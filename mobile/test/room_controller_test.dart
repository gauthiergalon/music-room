import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/controllers/room_controller.dart';
import 'package:mobile/core/exceptions/api_exception.dart';
import 'package:mobile/models/room.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/services/audio_service.dart';
import 'package:mobile/services/room_repository.dart';
import 'package:mobile/services/websocket_service.dart';

class _FakeEngine implements PlaybackEngine {
  bool playing = false;
  bool completed = false;
  Duration pos = Duration.zero;
  final List<String> calls = [];
  final StreamController<void> _completed = StreamController.broadcast(
    sync: true,
  );

  /// When set, `load` waits for it (a slow load).
  Completer<void>? loadGate;

  void finishTrack() {
    completed = true;
    _completed.add(null);
  }

  @override
  bool get isPlaying => playing;

  @override
  bool get isCompleted => completed;

  @override
  Duration get position => pos;

  @override
  Stream<Duration> get positionStream => const Stream.empty();

  @override
  Stream<Duration?> get durationStream => const Stream.empty();

  @override
  Stream<void> get completedStream => _completed.stream;

  @override
  Future<void> load(
    Track track,
    String streamUrl, {
    Duration initialPosition = Duration.zero,
  }) async {
    playing = false;
    await loadGate?.future;
    completed = false;
    pos = initialPosition;
    calls.add('load ${track.id} $streamUrl @${initialPosition.inMilliseconds}');
  }

  @override
  void play() {
    playing = true;
    calls.add('play');
  }

  @override
  void pause() {
    playing = false;
    calls.add('pause');
  }

  @override
  Future<void> seek(Duration position) async {
    pos = position;
    completed = false;
    calls.add('seek ${position.inMilliseconds}');
  }

  @override
  Future<void> stop() async {
    playing = false;
    completed = false;
    pos = Duration.zero;
    calls.add('stop');
  }
}

class _FakeSocket extends WebSocketService {
  _FakeSocket()
    : super(
        channelFactory: (_, _, _) => throw UnimplementedError(),
        tokenProvider: () async => 'token',
        baseUrl: () => 'http://backend.test',
        deviceName: () => 'Test device',
      );

  /// Events sent to the server, as `{type: payload}`.
  final List<Map<String, Map<String, dynamic>>> sent = [];

  List<Map<String, dynamic>> sentOf(String type) => [
    for (final event in sent)
      if (event.containsKey(type)) event[type]!,
  ];

  @override
  Future<void> connect(String roomId) async {
    state.value = WsConnectionState.connected;
  }

  @override
  Future<void> disconnect() async {
    state.value = WsConnectionState.disconnected;
  }

  @override
  void reconnectNow() {}

  @override
  void send(String eventType, Map<String, dynamic> payload) {
    sent.add({eventType: payload});
  }

  void emit(String type, [Map<String, dynamic> payload = const {}]) {
    onEvent?.call(type, payload);
  }
}

class _FakeRepository extends RoomRepository {
  /// Stream URL requests wait for the gate of their track, if any.
  final Map<int, Completer<void>> urlGates = {};
  Object? removeError;
  final List<(String, double)> reorders = [];

  @override
  Future<List<Room>> getRooms() async => [];

  @override
  Future<String> getStreamUrl(int trackId) async {
    await urlGates[trackId]?.future;
    return 'url-$trackId';
  }

  @override
  Future<void> removeQueueItem(String roomId, String queueItemId) async {
    final error = removeError;
    if (error != null) throw error;
  }

  @override
  Future<void> reorderQueueItem(
    String roomId,
    String queueItemId,
    double newPosition,
  ) async {
    reorders.add((queueItemId, newPosition));
  }
}

Map<String, dynamic> _trackJson(int id, {int duration = 180}) => {
  'id': id,
  'title': 'Track $id',
  'duration': duration,
  'artists': [
    {'name': 'Artist'},
  ],
};

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const me = 'user-me';
  const myConnection = 'conn-me';
  final t0 = DateTime.utc(2026, 1, 1, 12);

  late _FakeEngine engine;
  late _FakeSocket socket;
  late _FakeRepository repository;
  late RoomController controller;
  late DateTime now;
  late int version;

  /// Lets pending futures (URL fetch, load...) run.
  Future<void> settle() => Future<void>.delayed(Duration.zero);

  Map<String, dynamic> roomState({
    int? trackId = 1,
    bool isPlaying = true,
    int positionMs = 0,
    DateTime? timestamp,
    List<Map<String, dynamic>> queue = const [],
    int? withVersion,
  }) => {
    'current_track': trackId == null ? null : _trackJson(trackId),
    'version': withVersion ?? ++version,
    'is_playing': isPlaying,
    'current_position': positionMs,
    'timestamp': (timestamp ?? now).toIso8601String(),
    'server_time': now.toIso8601String(),
    'queue': queue,
  };

  Map<String, dynamic> queued(String id, int trackId, double position) => {
    'id': id,
    'position': position,
    'track': _trackJson(trackId),
  };

  Future<void> join({String owner = me}) async {
    await controller.openRoom(Room(id: 'room-1', owner: owner, name: 'Room'));
    socket.emit('Welcome', {
      'connection_id': myConnection,
      'server_time': now.toIso8601String(),
    });
  }

  setUp(() {
    engine = _FakeEngine();
    socket = _FakeSocket();
    repository = _FakeRepository();
    now = t0;
    version = 0;
    controller = RoomController(
      audioService: engine,
      wsService: socket,
      roomRepository: repository,
      now: () => now,
    )..setCurrentUserId(me);
  });

  tearDown(() => controller.dispose());

  group('joining a room', () {
    test(
      'a paused room is loaded where it was paused and stays paused',
      () async {
        await join();
        socket.emit(
          'RoomState',
          roomState(
            isPlaying: false,
            positionMs: 42000,
            // Paused long ago: that time must not be added to the position.
            timestamp: t0.subtract(const Duration(minutes: 5)),
          ),
        );
        await settle();

        expect(engine.calls, ['load 1 url-1 @42000']);
        expect(engine.playing, isFalse);
        expect(controller.isPlaying, isFalse);
      },
    );

    test('a playing room starts at the position it has reached', () async {
      await join();
      socket.emit(
        'RoomState',
        roomState(
          positionMs: 10000,
          timestamp: t0.subtract(const Duration(seconds: 20)),
        ),
      );
      await settle();

      expect(engine.calls, ['load 1 url-1 @30000', 'play']);
      expect(controller.isPlaying, isTrue);
    });

    test('the time spent fetching the stream is caught up', () async {
      final gate = repository.urlGates[1] = Completer<void>();
      await join();
      socket.emit('RoomState', roomState(positionMs: 0));
      await settle();
      expect(engine.calls, isEmpty);

      // The stream URL takes 4 seconds to arrive.
      now = t0.add(const Duration(seconds: 4));
      gate.complete();
      await settle();

      expect(engine.calls, ['load 1 url-1 @4000', 'play']);
    });
  });

  group('track changes', () {
    test('a slower earlier load never replaces a newer track', () async {
      final slow = repository.urlGates[1] = Completer<void>();
      await join();
      socket.emit('RoomState', roomState(trackId: 1));
      socket.emit('RoomState', roomState(trackId: 2));
      await settle();
      expect(engine.calls, ['load 2 url-2 @0', 'play']);

      slow.complete();
      await settle();

      expect(engine.calls, ['load 2 url-2 @0', 'play']);
      expect(controller.currentTrack?.id, 2);
    });

    test('a room without a track stops the player', () async {
      await join();
      socket.emit('RoomState', roomState());
      await settle();
      engine.calls.clear();

      socket.emit('RoomState', roomState(trackId: null, isPlaying: false));
      await settle();

      expect(engine.calls, ['stop']);
      expect(controller.currentTrack, isNull);
      expect(controller.isPlaying, isFalse);
    });

    test('an out-of-date RoomState is ignored', () async {
      await join();
      socket.emit('RoomState', roomState(trackId: 2, withVersion: 5));
      await settle();
      engine.calls.clear();

      socket.emit('RoomState', roomState(trackId: 1, withVersion: 4));
      await settle();

      expect(engine.calls, isEmpty);
      expect(controller.currentTrack?.id, 2);
    });

    test('versions start over after a reconnection', () async {
      await join();
      socket.emit('RoomState', roomState(trackId: 2, withVersion: 50));
      await settle();

      socket.emit('Welcome', {'connection_id': 'conn-2'});
      socket.emit('RoomState', roomState(trackId: 3, withVersion: 1));
      await settle();

      expect(controller.currentTrack?.id, 3);
    });
  });

  group('follow-up states of the same track', () {
    setUp(() async {
      await join(owner: 'someone-else');
      socket.emit('RoomState', roomState());
      await settle();
      engine.calls.clear();
    });

    test('a pause pauses the player', () async {
      now = t0.add(const Duration(seconds: 10));
      engine.pos = const Duration(seconds: 10);
      socket.emit(
        'RoomState',
        roomState(isPlaying: false, positionMs: 10000, timestamp: t0),
      );

      expect(engine.calls, ['pause']);
    });

    test('a seek moves the player', () async {
      socket.emit('RoomState', roomState(positionMs: 90000));

      expect(engine.calls, ['seek 90000']);
    });

    test('a small difference is left alone', () async {
      now = t0.add(const Duration(seconds: 10));
      engine.pos = const Duration(milliseconds: 10400);
      socket.emit('RoomState', roomState(positionMs: 0, timestamp: t0));

      expect(engine.calls, isEmpty);
    });
  });

  group('end of track', () {
    test('the controlling device asks for the next track once', () async {
      await join();
      socket.emit('RoomState', roomState(trackId: 1));
      await settle();

      engine.finishTrack();
      engine.finishTrack();

      expect(socket.sentOf('NextTrack'), [
        {'track_id': 1},
      ]);
    });

    test('it asks even when the queue is empty', () async {
      await join();
      socket.emit('RoomState', roomState(trackId: 1, queue: const []));
      await settle();

      engine.finishTrack();

      expect(socket.sentOf('NextTrack'), hasLength(1));
    });

    test('a listener does not ask for the next track', () async {
      await join(owner: 'someone-else');
      socket.emit('RoomState', roomState(trackId: 1));
      await settle();

      engine.finishTrack();

      expect(socket.sentOf('NextTrack'), isEmpty);
    });

    test(
      'a track that ended while offline is reported after reconnecting',
      () async {
        await join();
        socket.emit('RoomState', roomState(trackId: 1));
        await settle();

        socket.state.value = WsConnectionState.reconnecting;
        engine.finishTrack();
        expect(socket.sentOf('NextTrack'), isEmpty);

        // Back online, long after the end of the 3 minute track.
        now = t0.add(const Duration(minutes: 10));
        socket.state.value = WsConnectionState.connected;
        socket.emit('Welcome', {'connection_id': myConnection});
        socket.emit('RoomState', roomState(trackId: 1, timestamp: t0));

        expect(socket.sentOf('NextTrack'), [
          {'track_id': 1},
        ]);
      },
    );
  });

  group('canControl', () {
    void userState({String owner = me, String? delegate, String? connection}) {
      socket.emit('UserState', {
        'user_list': [
          {
            'user_id': me,
            'username': 'me',
            'connection_id': myConnection,
            'device_name': 'Phone',
          },
        ],
        'owner': owner,
        'delegate_user_id': delegate,
        'delegate_connection_id': connection,
      });
    }

    test('the owner controls a room without delegate', () async {
      await join();
      userState();
      expect(controller.canControl, isTrue);
    });

    test('a listener does not', () async {
      await join(owner: 'someone-else');
      userState(owner: 'someone-else');
      expect(controller.canControl, isFalse);
    });

    test('the owner loses control while a delegate is set', () async {
      await join();
      userState(delegate: 'other', connection: 'conn-other');
      expect(controller.canControl, isFalse);
    });

    test('only the delegated device of the delegate controls', () async {
      await join(owner: 'someone-else');

      userState(owner: 'someone-else', delegate: me, connection: myConnection);
      expect(controller.canControl, isTrue);

      userState(owner: 'someone-else', delegate: me, connection: 'my-tablet');
      expect(controller.canControl, isFalse);
    });

    test('nobody controls while the connection is down', () async {
      await join();
      socket.state.value = WsConnectionState.reconnecting;
      expect(controller.canControl, isFalse);
    });
  });

  group('controls', () {
    setUp(() async {
      await join();
      socket.emit('RoomState', roomState());
      await settle();
      engine.calls.clear();
      socket.sent.clear();
    });

    test('pause reports the position and pauses locally', () {
      engine.pos = const Duration(seconds: 12);

      controller.togglePlay();

      expect(engine.calls, ['pause']);
      expect(socket.sent, [
        {
          'Pause': {'position': 12000},
        },
      ]);
      expect(controller.isPlaying, isFalse);
    });

    test('play reports the position without a client timestamp', () {
      controller.togglePlay();
      engine.calls.clear();
      socket.sent.clear();
      engine.pos = const Duration(seconds: 12);

      controller.togglePlay();

      expect(engine.calls, ['play']);
      expect(socket.sent, [
        {
          'Play': {'position': 12000},
        },
      ]);
      expect(controller.isPlaying, isTrue);
    });

    test('seek moves the player and tells the server', () {
      controller.seekTo(const Duration(seconds: 50));

      expect(engine.calls, ['seek 50000']);
      expect(socket.sent, [
        {
          'SeekTo': {'position': 50000},
        },
      ]);
    });

    test('the answer to an older command does not move the player back', () {
      controller.seekTo(const Duration(seconds: 50));
      engine.calls.clear();

      // The server still answers to something sent before the seek.
      socket.emit('RoomState', roomState(positionMs: 5000));

      expect(engine.calls, isEmpty);
    });

    test('skip asks for the track after the current one', () {
      controller.skipNext();

      expect(socket.sent, [
        {
          'NextTrack': {'track_id': 1},
        },
      ]);
    });

    test('a listener cannot control', () async {
      socket.emit('UserState', {
        'user_list': const <dynamic>[],
        'owner': 'someone-else',
        'delegate_user_id': null,
        'delegate_connection_id': null,
      });

      controller.togglePlay();
      controller.seekTo(const Duration(seconds: 5));
      controller.skipNext();

      expect(engine.calls, isEmpty);
      expect(socket.sent, isEmpty);
    });
  });

  group('queue', () {
    setUp(() async {
      await join();
      socket.emit(
        'RoomState',
        roomState(
          queue: [queued('a', 10, 0), queued('b', 11, 1), queued('c', 12, 2)],
        ),
      );
      await settle();
    });

    test('carries the tracks sent by the server', () {
      expect(controller.queue.map((q) => q.track.title), [
        'Track 10',
        'Track 11',
        'Track 12',
      ]);
    });

    test('an item is removed right away', () async {
      final removal = controller.removeQueueItem(controller.queue[1]);

      expect(controller.queue.map((q) => q.id), ['a', 'c']);
      await removal;
      expect(controller.queue.map((q) => q.id), ['a', 'c']);
    });

    test('a failed removal puts the item back', () async {
      repository.removeError = ApiException('Server error', 500);

      await expectLater(
        controller.removeQueueItem(controller.queue[1]),
        throwsA(isA<ApiException>()),
      );

      expect(controller.queue.map((q) => q.id), ['a', 'b', 'c']);
    });

    test('an item that was already gone stays removed', () async {
      repository.removeError = ApiException('Not found', 404);

      await controller.removeQueueItem(controller.queue[1]);

      expect(controller.queue.map((q) => q.id), ['a', 'c']);
    });

    test(
      'moving an item sends a position between its new neighbours',
      () async {
        await controller.reorderQueueItem(2, 1);

        expect(controller.queue.map((q) => q.id), ['a', 'c', 'b']);
        final (id, position) = repository.reorders.single;
        expect(id, 'c');
        expect(position, greaterThan(0));
        expect(position, lessThan(1));
      },
    );
  });

  group('leaving', () {
    test('leaving stops the player and the connection', () async {
      await join();
      socket.emit('RoomState', roomState());
      await settle();
      engine.calls.clear();

      controller.leaveRoom();

      expect(engine.calls, ['stop']);
      expect(controller.currentRoom, isNull);
      expect(socket.state.value, WsConnectionState.disconnected);
    });

    test('a closed room leaves and tells why', () async {
      await join(owner: 'someone-else');

      socket.emit('RoomClosed');

      expect(controller.currentRoom, isNull);
      expect(controller.takeRoomClosedMessage(), 'Room closed by owner');
      expect(controller.takeRoomClosedMessage(), isNull);
    });

    test('a load that finishes after leaving does not start playing', () async {
      final gate = repository.urlGates[1] = Completer<void>();
      await join();
      socket.emit('RoomState', roomState());
      await settle();

      controller.leaveRoom();
      engine.calls.clear();
      gate.complete();
      await settle();

      expect(engine.calls, isEmpty);
    });

    test('reset forgets the rooms of the user', () async {
      await join();

      controller.reset();

      expect(controller.currentRoom, isNull);
      expect(controller.availableRooms, isEmpty);
    });
  });
}
