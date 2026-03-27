import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel createWsChannel(String baseUrl, String token) {
  final uri = Uri.parse('$baseUrl?token=$token');
  return WebSocketChannel.connect(uri);
}
