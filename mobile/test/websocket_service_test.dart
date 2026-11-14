import 'dart:async';
import 'dart:math';

import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/services/websocket_service.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

class _FakeSink implements WebSocketSink {
  final List<dynamic> sent = [];
  bool closed = false;

  @override
  void add(dynamic data) => sent.add(data);

  @override
  Future<dynamic> close([int? closeCode, String? closeReason]) async {
    closed = true;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakeChannel implements WebSocketChannel {
  final StreamController<dynamic> incoming = StreamController<dynamic>();
  final Completer<void> handshake = Completer<void>();
  final _FakeSink fakeSink = _FakeSink();

  /// Completes the handshake.
  void open() => handshake.complete();

  /// Fails the handshake, like a refused upgrade.
  void refuse() {
    handshake.completeError(WebSocketChannelException('not upgraded'));
    incoming.close();
  }

  /// The server (or the network) closes an established connection.
  void drop() => incoming.close();

  @override
  Future<void> get ready => handshake.future;

  @override
  Stream<dynamic> get stream => incoming.stream;

  @override
  WebSocketSink get sink => fakeSink;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FixedRandom implements Random {
  @override
  double nextDouble() => 0.5;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  late List<_FakeChannel> channels;
  late List<String> urls;
  late String? token;
  late WebSocketService service;
  late List<Map<String, Map<String, dynamic>>> events;
  late int unavailableCalls;

  setUp(() {
    channels = [];
    urls = [];
    token = 'access-token';
    events = [];
    unavailableCalls = 0;
    service = WebSocketService(
      channelFactory: (url, token, device) {
        urls.add(url);
        final channel = _FakeChannel();
        channels.add(channel);
        return channel;
      },
      tokenProvider: () async => token,
      baseUrl: () => 'https://backend.test',
      deviceName: () => 'Test device',
      random: _FixedRandom(),
    );
    service.onEvent = (type, payload) => events.add({type: payload});
    service.onRoomUnavailable = () => unavailableCalls++;
  });

  tearDown(() => service.dispose());

  group('reconnectDelay', () {
    test('doubles from one second and is capped at fifteen', () {
      final delays = [
        for (var attempt = 0; attempt < 7; attempt++)
          reconnectDelay(attempt).inMilliseconds,
      ];
      expect(delays, [1000, 2000, 4000, 8000, 15000, 15000, 15000]);
    });

    test('is spread by twenty percent around the base', () {
      expect(reconnectDelay(0, jitter: 0).inMilliseconds, 800);
      expect(reconnectDelay(0, jitter: 1).inMilliseconds, 1200);
    });
  });

  testWidgets('is connected only once the handshake is done', (tester) async {
    unawaited(service.connect('room-1'));
    await tester.pump();

    expect(urls, ['wss://backend.test/rooms/room-1/ws']);
    expect(service.state.value, WsConnectionState.connecting);

    channels.single.open();
    await tester.pump();

    expect(service.state.value, WsConnectionState.connected);
    expect(service.isConnected, isTrue);
  });

  testWidgets('delivers events and ignores malformed messages', (tester) async {
    unawaited(service.connect('room-1'));
    await tester.pump();
    channels.single.open();
    await tester.pump();

    channels.single.incoming
      ..add('not json')
      ..add('[1, 2]')
      ..add('{"type":"RoomClosed"}')
      ..add('{"type":"Welcome","payload":{"connection_id":"c1"}}');
    await tester.pump();

    expect(events, [
      {'RoomClosed': <String, dynamic>{}},
      {
        'Welcome': {'connection_id': 'c1'},
      },
    ]);
    expect(service.state.value, WsConnectionState.connected);
  });

  testWidgets('reconnects after the connection drops', (tester) async {
    unawaited(service.connect('room-1'));
    await tester.pump();
    channels.single.open();
    await tester.pump();

    channels.single.drop();
    await tester.pump();
    expect(service.state.value, WsConnectionState.reconnecting);
    expect(channels, hasLength(1));

    await tester.pump(const Duration(seconds: 1));
    expect(channels, hasLength(2));

    channels.last.open();
    await tester.pump();
    expect(service.state.value, WsConnectionState.connected);
  });

  testWidgets('waits longer after each failed attempt', (tester) async {
    service.roomProbe = (_) async => true;
    unawaited(service.connect('room-1'));
    await tester.pump();

    channels.last.refuse();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 999));
    expect(channels, hasLength(1));
    await tester.pump(const Duration(milliseconds: 1));
    expect(channels, hasLength(2));

    channels.last.refuse();
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1999));
    expect(channels, hasLength(2));
    await tester.pump(const Duration(milliseconds: 1));
    expect(channels, hasLength(3));

    expect(unavailableCalls, 0);
    expect(service.state.value, WsConnectionState.reconnecting);
  });

  testWidgets('gives up when the room is gone', (tester) async {
    final probed = <String>[];
    service.roomProbe = (roomId) async {
      probed.add(roomId);
      return false;
    };
    unawaited(service.connect('room-1'));
    await tester.pump();

    channels.single.refuse();
    await tester.pump();
    await tester.pump(const Duration(seconds: 30));

    expect(probed, ['room-1']);
    expect(unavailableCalls, 1);
    expect(channels, hasLength(1));
    expect(service.state.value, WsConnectionState.disconnected);
  });

  testWidgets('keeps retrying when the room cannot be checked', (tester) async {
    service.roomProbe = (_) async => throw Exception('offline');
    unawaited(service.connect('room-1'));
    await tester.pump();

    channels.single.refuse();
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));

    expect(channels, hasLength(2));
    expect(unavailableCalls, 0);
  });

  testWidgets('does not connect without a session', (tester) async {
    token = null;
    unawaited(service.connect('room-1'));
    await tester.pump();
    await tester.pump(const Duration(seconds: 30));

    expect(channels, isEmpty);
    expect(service.state.value, WsConnectionState.disconnected);
  });

  testWidgets('disconnect stops the reconnection', (tester) async {
    unawaited(service.connect('room-1'));
    await tester.pump();
    channels.single.open();
    await tester.pump();
    channels.single.drop();
    await tester.pump();

    unawaited(service.disconnect());
    await tester.pump(const Duration(seconds: 30));

    expect(channels, hasLength(1));
    expect(service.state.value, WsConnectionState.disconnected);
  });

  testWidgets('reconnectNow skips the wait', (tester) async {
    service.roomProbe = (_) async => true;
    unawaited(service.connect('room-1'));
    await tester.pump();
    channels.single.refuse();
    await tester.pump();
    expect(channels, hasLength(1));

    service.reconnectNow();
    await tester.pump();

    expect(channels, hasLength(2));
  });

  testWidgets('only sends while connected', (tester) async {
    service.send('Play', {'position': 1});

    unawaited(service.connect('room-1'));
    await tester.pump();
    channels.single.open();
    await tester.pump();
    service.send('Pause', {'position': 2});

    expect(channels.single.fakeSink.sent, [
      '{"type":"Pause","payload":{"position":2}}',
    ]);
  });
}
