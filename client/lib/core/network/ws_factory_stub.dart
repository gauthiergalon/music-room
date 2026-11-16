import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel createWsChannel(
  String baseUrl,
  String token,
  String deviceName,
) {
  throw UnsupportedError(
    'Cannot create a WebSocket without dart:io or dart:html',
  );
}
