import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../core/logger/logger.dart';
import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';

enum WsConnectionState { disconnected, connecting, connected, reconnecting }

typedef WsEventCallback =
    void Function(String type, Map<String, dynamic> payload);

typedef WsChannelFactory =
    WebSocketChannel Function(String url, String token, String deviceName);

/// Delay before reconnection attempt number [attempt] (0-based): 1 s, 2 s,
/// 4 s ... capped at 15 s, spread by +/-20 % so clients don't retry in step.
Duration reconnectDelay(int attempt, {double jitter = 0.5}) {
  final baseMs = min(15000, 1000 * (1 << min(attempt, 4)));
  final factor = 0.8 + 0.4 * jitter.clamp(0.0, 1.0);
  return Duration(milliseconds: (baseMs * factor).round());
}

/// WebSocket connection to one room, kept alive until [disconnect].
///
/// When the socket drops, it reconnects with an exponential backoff. If a
/// connection attempt fails and the room turns out to be gone or forbidden,
/// it gives up and calls [onRoomUnavailable].
class WebSocketService {
  WebSocketService({
    WsChannelFactory? channelFactory,
    Future<String?> Function()? tokenProvider,
    String Function()? baseUrl,
    String Function()? deviceName,
    Random? random,
  }) : _channelFactory = channelFactory ?? createWsChannel,
       _tokenProvider = tokenProvider ?? ApiClient.ensureFreshToken,
       _baseUrl = baseUrl ?? (() => ApiClient.baseUrl),
       _deviceName = deviceName ?? (() => ApiClient.deviceName),
       _random = random ?? Random();

  final WsChannelFactory _channelFactory;
  final Future<String?> Function() _tokenProvider;
  final String Function() _baseUrl;
  final String Function() _deviceName;
  final Random _random;

  final ValueNotifier<WsConnectionState> state = ValueNotifier(
    WsConnectionState.disconnected,
  );

  WsEventCallback? onEvent;

  /// The room no longer exists or the user lost access to it.
  VoidCallback? onRoomUnavailable;

  /// Tells whether the room can still be joined: `true`/`false`, or throws
  /// when that cannot be determined (e.g. no network).
  Future<bool> Function(String roomId)? roomProbe;

  String? _roomId;
  WebSocketChannel? _channel;
  StreamSubscription<dynamic>? _subscription;
  Timer? _retryTimer;
  int _attempt = 0;

  /// Identifies the current connection attempt. Anything still running for
  /// an older attempt sees a different value and stops.
  int _session = 0;
  bool _disposed = false;

  bool get isConnected => state.value == WsConnectionState.connected;

  /// Opens the connection to [roomId] and keeps it open.
  Future<void> connect(String roomId) async {
    _close();
    _roomId = roomId;
    _attempt = 0;
    await _open();
  }

  /// Retries immediately instead of waiting for the backoff (app resumed).
  void reconnectNow() {
    if (_roomId == null || _disposed) return;
    if (state.value == WsConnectionState.connected ||
        state.value == WsConnectionState.connecting) {
      return;
    }
    _retryTimer?.cancel();
    _attempt = 0;
    unawaited(_open());
  }

  Future<void> disconnect() async {
    _roomId = null;
    _close();
    _setState(WsConnectionState.disconnected);
  }

  void send(String eventType, Map<String, dynamic> payload) {
    final channel = _channel;
    if (channel == null || !isConnected) {
      logger.warning('WS not connected, cannot send: $eventType');
      return;
    }

    try {
      channel.sink.add(jsonEncode({'type': eventType, 'payload': payload}));
      logger.debug('WS send: $eventType');
    } catch (e) {
      logger.error('Error sending WS event', error: e);
    }
  }

  void dispose() {
    _disposed = true;
    _roomId = null;
    _close();
    state.dispose();
  }

  Future<void> _open() async {
    final roomId = _roomId;
    if (roomId == null || _disposed) return;

    final session = ++_session;
    // A previous attempt may still hold a socket (e.g. `reconnectNow` while
    // a handshake was in progress).
    _releaseChannel();
    _setState(
      _attempt == 0
          ? WsConnectionState.connecting
          : WsConnectionState.reconnecting,
    );

    try {
      final token = await _tokenProvider();
      if (session != _session) return;
      if (token == null) {
        // No session any more: the logout flow takes over.
        _roomId = null;
        _setState(WsConnectionState.disconnected);
        return;
      }

      final url = '${_baseUrl().replaceFirst('http', 'ws')}/rooms/$roomId/ws';
      logger.debug('WS connecting to $url');
      final channel = _channelFactory(url, token, _deviceName());
      _channel = channel;

      // Until the handshake is done, a failure is reported by `ready` below
      // and handled there; the stream callbacks only matter afterwards.
      var established = false;
      var closedEarly = false;
      void onClosed(Object? error) {
        if (established) {
          _onDropped(session, error);
        } else {
          closedEarly = true;
        }
      }

      _subscription = channel.stream.listen(
        _handleMessage,
        onError: (Object error) => onClosed(error),
        onDone: () => onClosed(null),
      );

      await channel.ready;
      if (session != _session) return;

      established = true;
      _attempt = 0;
      _setState(WsConnectionState.connected);
      logger.info('WS connected');
      if (closedEarly) _onDropped(session, null);
    } catch (e) {
      await _onConnectFailed(session, roomId, e);
    }
  }

  /// The handshake failed: find out whether retrying makes sense.
  Future<void> _onConnectFailed(
    int session,
    String roomId,
    Object error,
  ) async {
    if (session != _session) return;
    logger.error('WS connection failed', error: error);
    // Invalidate this attempt so the stream's own error/done callbacks, which
    // fire for the same failure, are ignored.
    _session++;
    _releaseChannel();

    final probe = roomProbe;
    if (probe != null) {
      try {
        final available = await probe(roomId);
        if (_roomId != roomId || _disposed) return;
        if (!available) {
          _roomId = null;
          _setState(WsConnectionState.disconnected);
          onRoomUnavailable?.call();
          return;
        }
      } catch (_) {
        // Unknown (offline?): keep retrying.
      }
    }

    if (_roomId != roomId || _disposed) return;
    _scheduleReconnect();
  }

  /// An established connection closed.
  void _onDropped(int session, Object? error) {
    if (session != _session) return;
    _session++;
    if (error != null) {
      logger.error('WS error', error: error);
    } else {
      logger.info('WS disconnected');
    }
    _releaseChannel();
    if (_roomId == null || _disposed) {
      _setState(WsConnectionState.disconnected);
      return;
    }
    _scheduleReconnect();
  }

  void _scheduleReconnect() {
    final delay = reconnectDelay(_attempt, jitter: _random.nextDouble());
    _attempt++;
    _setState(WsConnectionState.reconnecting);
    logger.info('WS reconnecting in ${delay.inMilliseconds} ms');
    _retryTimer?.cancel();
    _retryTimer = Timer(delay, () => unawaited(_open()));
  }

  void _handleMessage(dynamic message) {
    if (message is! String) return;

    final Object? data;
    try {
      data = jsonDecode(message);
    } catch (e) {
      logger.warning('WS received malformed JSON');
      return;
    }
    if (data is! Map) return;

    final type = data['type']?.toString() ?? '';
    final rawPayload = data['payload'];
    final payload = rawPayload is Map
        ? Map<String, dynamic>.from(rawPayload)
        : <String, dynamic>{};

    logger.debug('WS recv: $type');
    try {
      onEvent?.call(type, payload);
    } catch (e, st) {
      logger.error('Error handling WS event $type', error: e, stackTrace: st);
    }
  }

  void _close() {
    _session++;
    _retryTimer?.cancel();
    _retryTimer = null;
    _releaseChannel();
  }

  /// Drops the current socket. Nothing is awaited: closing a socket whose
  /// handshake never finished may never complete, and nothing depends on it.
  void _releaseChannel() {
    final subscription = _subscription;
    final channel = _channel;
    _subscription = null;
    _channel = null;
    if (subscription != null) unawaited(subscription.cancel());
    if (channel == null) return;
    try {
      unawaited(channel.sink.close().then<void>((_) {}, onError: (_) {}));
    } catch (_) {
      // Nothing to clean up.
    }
  }

  void _setState(WsConnectionState value) {
    if (_disposed) return;
    state.value = value;
  }
}
