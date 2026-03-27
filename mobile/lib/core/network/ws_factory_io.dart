import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel createWsChannel(String baseUrl, String token) {
  return IOWebSocketChannel.connect(
    Uri.parse(baseUrl),
    headers: {'Authorization': 'Bearer $token'},
  );
}
