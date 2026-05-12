import 'dart:async';
import 'dart:convert';

import 'package:flutter/widgets.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';

typedef WsEventCallback = void Function(String type, Map<String, dynamic> payload);

class WebSocketService extends ChangeNotifier {
  WebSocketChannel? _channel;
  StreamSubscription? _subscription;
  bool _isConnected = false;
  WsEventCallback? _onEvent;

  bool get isConnected => _isConnected;

  void setEventCallback(WsEventCallback callback) {
    _onEvent = callback;
  }

  Future<void> connect(String roomId) async {
    await disconnect();

    final token = await ApiClient.getToken();
    if (token == null) return;

    final baseUrl =
        '${ApiClient.baseUrl.replaceFirst('http', 'ws')}/rooms/$roomId/ws';

    try {
      _channel = createWsChannel(baseUrl, token);
      _subscription = _channel!.stream.listen(
        _handleMessage,
        onError: (err) => debugPrint('WebSocket error: $err'),
        onDone: () => _isConnected = false,
      );
      _isConnected = true;
      notifyListeners();
    } catch (e) {
      debugPrint('Failed to connect to WebSocket: $e');
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

    _onEvent?.call(type, payload);
  }

  void send(String eventType, Map<String, dynamic> payload) {
    if (_channel == null) {
      debugPrint('WebSocket not connected, cannot send event: $eventType');
      return;
    }

    try {
      final event = {'type': eventType, 'payload': payload};
      _channel!.sink.add(jsonEncode(event));
    } catch (e) {
      debugPrint('Error sending WS event: $e');
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
