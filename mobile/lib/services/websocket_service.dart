import 'dart:async';
import 'dart:convert';

import 'package:flutter/widgets.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../core/logger/logger.dart';
import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';

class WebSocketService extends ChangeNotifier {
  WebSocketChannel? _channel;
  StreamSubscription? _subscription;
  bool _isConnected = false;
  Function(String type, Map<String, dynamic> payload)? _onEvent;

  bool get isConnected => _isConnected;

  void setEventCallback(
    Function(String type, Map<String, dynamic> payload) callback,
  ) {
    _onEvent = callback;
  }

  Future<void> connect(String roomId) async {
    await disconnect();

    final token = await ApiClient.getToken();
    if (token == null) return;

    final baseUrl =
        '${ApiClient.baseUrl.replaceFirst('http', 'ws')}/rooms/$roomId/ws';

    try {
      logger.debug('WS connecting to $baseUrl');
      _channel = createWsChannel(baseUrl, token);
      _subscription = _channel!.stream.listen(
        _handleMessage,
        onError: (err) => logger.error('WS error', error: err),
        onDone: () {
          _isConnected = false;
          logger.info('WS disconnected');
          notifyListeners();
        },
      );
      _isConnected = true;
      logger.info('WS connected');
      notifyListeners();
    } catch (e) {
      logger.error('Failed to connect to WebSocket', error: e);
      _isConnected = false;
      notifyListeners();
    }
  }

  void _handleMessage(dynamic message) {
    final data = jsonDecode(message as String);
    if (data is! Map<String, dynamic>) return;

    final type = data['type']?.toString() ?? '';
    final payload = Map<String, dynamic>.from(
      data['payload'] as Map? ?? const {},
    );

    logger.debug('WS recv: $type');
    _onEvent?.call(type, payload);
  }

  void send(String eventType, Map<String, dynamic> payload) {
    if (_channel == null) {
      logger.warning('WS not connected, cannot send: $eventType');
      return;
    }

    try {
      final event = {'type': eventType, 'payload': payload};
      _channel!.sink.add(jsonEncode(event));
      logger.debug('WS send: $eventType');
    } catch (e) {
      logger.error('Error sending WS event', error: e);
    }
  }

  Future<void> disconnect() async {
    await _subscription?.cancel();
    _subscription = null;
    _channel?.sink.close();
    _channel = null;
    _isConnected = false;
    notifyListeners();
  }

  @override
  void dispose() {
    disconnect();
    super.dispose();
  }
}
