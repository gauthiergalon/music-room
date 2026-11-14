import 'package:web_socket_channel/io.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel createWsChannel(
  String baseUrl,
  String token,
  String deviceName,
) {
  return IOWebSocketChannel.connect(
    Uri.parse(baseUrl),
    headers: {'Authorization': 'Bearer $token', 'X-Device': deviceName},
    // Closes the socket when the server stops answering pings, so a dead
    // connection is noticed and the reconnection can start.
    pingInterval: const Duration(seconds: 20),
    connectTimeout: const Duration(seconds: 10),
  );
}
