import 'package:web_socket_channel/web_socket_channel.dart';

WebSocketChannel createWsChannel(
  String baseUrl,
  String token,
  String deviceName,
) {
  final uri = Uri.parse(
    baseUrl,
  ).replace(queryParameters: {'token': token, 'device': deviceName});
  return WebSocketChannel.connect(uri);
}
