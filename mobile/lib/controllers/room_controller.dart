import 'dart:async';
import 'dart:convert';

import 'package:flutter/widgets.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';
import '../models/queue_item.dart';
import '../models/room.dart';
import '../models/room_user.dart';
import '../models/track.dart';

class RoomController extends ChangeNotifier with WidgetsBindingObserver {
  static const String _silenceTrackId = 'silence_placeholder';
  static const String _silenceAssetPath = 'assets/audio/silence.mp3';
  static const Duration _silencePrimeDelay = Duration(seconds: 1);

  static const String _eventRoomState = 'RoomState';
  static const String _eventUserState = 'UserState';
  static const String _eventRoomClosed = 'RoomClosed';
  static const String _eventError = 'Error';

  static const String _eventPlay = 'Play';
  static const String _eventPause = 'Pause';
  static const String _eventSeekTo = 'SeekTo';
  static const String _eventNextTrack = 'NextTrack';

  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;
  Track? get currentTrack => _currentRoom?.currentTrack;
  bool get isPlaying => _audioPlayer.playing;
  Duration get playbackPosition => _audioPlayer.position;
  Duration? get playbackDuration => _audioPlayer.duration;

  List<Room> _availableRooms = [];
  List<Room> get availableRooms => _availableRooms;

  WebSocketChannel? _wsChannel;
  StreamSubscription? _wsSubscription;
  late final StreamSubscription<PlayerState> _playerStateSubscription;
  final AudioPlayer _audioPlayer = AudioPlayer();

  RoomController() {
    WidgetsBinding.instance.addObserver(this);

    _playerStateSubscription = _audioPlayer.playerStateStream.listen(
      _handlePlayerState,
    );
  }

  void _handlePlayerState(PlayerState state) {
    if (state.processingState == ProcessingState.completed) {
      _playNextInQueue();
    }

    notifyListeners();
  }

  void _playNextInQueue() {
    final room = _currentRoom;
    if (room != null && room.queue.isNotEmpty) {
      _sendWsEvent(_eventNextTrack, {
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    } else {
      _playSilenceBackground();
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _wsSubscription?.cancel();
    _wsChannel?.sink.close();
    _playerStateSubscription.cancel();
    _audioPlayer.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      if (_currentRoom != null && _wsChannel?.closeCode != null) {
        _checkAndReconnect();
      }
    }
  }

  Future<void> _checkAndReconnect() async {
    if (_currentRoom == null) return;
    try {
      await ApiClient.get('/rooms/${_currentRoom!.id}');

      await _connectWebSocket(_currentRoom!.id);
    } catch (e) {
      leaveRoom();
    }
  }

  Future<void> refreshRooms() async {
    try {
      final response = await ApiClient.get('/rooms');
      if (response is List) {
        _availableRooms = response.map((data) => Room.fromJson(data)).toList();
        notifyListeners();
      }
    } catch (e) {
      debugPrint('Failed to refresh rooms: $e');
      rethrow;
    }
  }

  Future<Room> createRoom() async {
    try {
      final response = await ApiClient.post('/rooms');
      final newRoom = Room.fromJson(response);
      _availableRooms.add(newRoom);
      notifyListeners();
      await openRoom(newRoom);
      return newRoom;
    } catch (e) {
      debugPrint('Failed to create room: $e');
      rethrow;
    }
  }

  Future<void> openRoom(Room room) async {
    _currentRoom = room;
    await _connectWebSocket(room.id);

    _playSilenceBackground();

    notifyListeners();
  }

  Future<void> _playSilenceBackground() async {
    try {
      await _audioPlayer.stop();

      if (_currentRoom != null) {
        _currentRoom!.currentTrack = null;
        _currentRoom!.status = 0;
        notifyListeners();
      }

      final silenceSource = AudioSource.asset(
        _silenceAssetPath,
        tag: const MediaItem(
          id: _silenceTrackId,
          title: 'Music Room Active',
          artist: 'Waiting for a track...',
        ),
      );

      await _audioPlayer.setLoopMode(LoopMode.one);
      await _audioPlayer.setAudioSource(silenceSource);
      _audioPlayer.play();
      await Future.delayed(_silencePrimeDelay);
      await _audioPlayer.pause();
    } catch (e) {
      debugPrint('Failed to start silence background playback: $e');
    }
  }

  void leaveRoom() {
    unawaited(_audioPlayer.stop());
    _currentRoom = null;
    _disposeWebSocket();
    notifyListeners();

    unawaited(refreshRooms());
  }

  void _disposeWebSocket() {
    _wsSubscription?.cancel();
    _wsSubscription = null;
    _wsChannel?.sink.close();
    _wsChannel = null;
  }

  Future<void> _connectWebSocket(String roomId) async {
    final token = await ApiClient.getToken();
    if (token == null) return;

    _disposeWebSocket();

    final baseUrl =
        '${ApiClient.baseUrl.replaceFirst('http', 'ws')}/rooms/$roomId/ws';
    try {
      _wsChannel = createWsChannel(baseUrl, token);
      _wsSubscription = _wsChannel!.stream.listen(
        (message) {
          final data = jsonDecode(message as String);
          if (data is Map<String, dynamic>) {
            _handleWsEvent(data);
          }
        },
        onError: (err) {
          debugPrint('WebSocket error: $err');
        },
      );
    } catch (e) {
      debugPrint('Failed to connect to WebSocket: $e');
      _disposeWebSocket();
      if (_currentRoom?.id == roomId) {
        leaveRoom();
      }
    }
  }

  void _sendWsEvent(String eventType, Map<String, dynamic> payload) {
    if (_wsChannel == null) {
      debugPrint('WebSocket not connected, cannot send event: $eventType');
      return;
    }
    try {
      final event = {'type': eventType, 'payload': payload};
      _wsChannel!.sink.add(jsonEncode(event));
    } catch (e) {
      debugPrint('Error sending WS event: $e');
    }
  }

  void _handleWsEvent(Map<String, dynamic> data) {
    if (_currentRoom == null) return;

    final type = data['type']?.toString();
    final payload = Map<String, dynamic>.from(
      data['payload'] as Map? ?? const {},
    );

    switch (type) {
      case _eventRoomState:
        _handleRoomState(payload);
        break;
      case _eventUserState:
        _handleUserState(payload);
        break;
      case _eventRoomClosed:
        leaveRoom();
        return;
      case _eventError:
        debugPrint('WebSocket Server Error: ${payload['message']}');
        break;
    }

    notifyListeners();
  }

  void _handleRoomState(Map<String, dynamic> payload) {
    if (payload['queue'] is List) {
      final queueList = payload['queue'] as List;
      _currentRoom!.queue = queueList.asMap().entries.map<QueueItem>((entry) {
        final trackJson = entry.value as Map<String, dynamic>;
        // Parse and cache full track metadata before creating QueueItem.
        Track.fromJson(trackJson);
        return QueueItem(
          id: trackJson['id'].toString(),
          roomId: _currentRoom!.id,
          trackId: trackJson['id'],
          position: entry.key.toDouble(),
        );
      }).toList();
    }

    if (payload['current_track'] != null) {
      final track = Track.fromJson(payload['current_track']);
      final bool trackChanged = currentTrack?.id != track.id;
      _currentRoom!.currentTrack = track;
      if (trackChanged) {
        playTrack(track);
      }
    } else {
      _currentRoom!.currentTrack = null;
      _playSilenceBackground();
    }

    final isPlaying = payload['is_playing'] == true;
    _currentRoom!.status = isPlaying ? 1 : 0;
    if (isPlaying && !_audioPlayer.playing) {
      _audioPlayer.play();
    } else if (!isPlaying && _audioPlayer.playing) {
      _audioPlayer.pause();
    }

    if (payload['current_position'] != null) {
      int posMs = payload['current_position'] as int;
      if (isPlaying && payload['timestamp'] != null) {
        final playedAt = DateTime.parse(
          payload['timestamp'].toString(),
        ).toUtc();
        final now = DateTime.now().toUtc();
        final diff = now.difference(playedAt).inMilliseconds;
        if (diff > 0) {
          posMs += diff;
        }
      }
      final position = Duration(milliseconds: posMs);

      if ((_audioPlayer.position - position).inMilliseconds.abs() > 2000) {
        _audioPlayer.seek(position);
      }
    }
  }

  void _handleUserState(Map<String, dynamic> payload) {
    final users = payload['user_list'];
    if (users is! List) return;

    _currentRoom!.listeners
      ..clear()
      ..addAll(
        users.map((user) => _roomUserFromPayload(user)).whereType<RoomUser>(),
      );

    final ownerId = payload['owner']?.toString();
    if (ownerId != null) {
      _currentRoom!.owner = ownerId;
    }
  }

  RoomUser? _roomUserFromPayload(dynamic payload) {
    if (payload is! Map) return null;

    final id = payload['user_id']?.toString();
    final username = payload['username']?.toString();

    if (id == null || username == null) return null;
    return RoomUser(id: id, username: username);
  }

  Future<String> _resolveStreamUrl(int trackId) async {
    final response = await ApiClient.get('/hifi/track/$trackId/stream-url');
    final streamUrl = response['stream_url'] as String?;

    if (streamUrl == null || streamUrl.isEmpty) {
      throw Exception('No stream URL found for track $trackId');
    }

    return streamUrl;
  }

  Future<void> addTrack(Room room, Track track) async {
    await ApiClient.post(
      '/rooms/${room.id}/queue',
      body: {'track_id': track.id},
    );
  }

  Future<void> removeQueueItem(Room room, QueueItem item) async {
    await ApiClient.delete('/rooms/${room.id}/queue', body: {'id': item.id});
  }

  Future<void> moveQueueItem(
    Room room,
    QueueItem item,
    double newPosition,
  ) async {
    await ApiClient.patch(
      '/rooms/${room.id}/queue',
      body: {'id': item.id, 'new_position': newPosition},
    );
  }

  Future<void> reorderQueueItem(
    Room room,
    List<QueueItem> queue,
    int oldIndex,
    int newIndex,
  ) async {
    if (newIndex > oldIndex) newIndex -= 1;

    final item = queue[oldIndex];
    double newPos;

    if (queue.isEmpty) {
      newPos = 0;
    } else if (newIndex == 0) {
      newPos = queue.first.position - 100;
    } else if (newIndex >= queue.length - 1) {
      newPos = queue.last.position + 100;
    } else {
      final prev = queue[newIndex - 1].position;
      final next = queue[newIndex].position;
      newPos = (prev + next) / 2;
    }

    await moveQueueItem(room, item, newPos);
  }

  void togglePlay(Room room) {
    if (_audioPlayer.playing) {
      _sendWsEvent(_eventPause, {
        'position': _audioPlayer.position.inMilliseconds,
      });
    } else {
      _sendWsEvent(_eventPlay, {
        'position': _audioPlayer.position.inMilliseconds,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    }
  }

  Future<void> playTrack(Track track) async {
    try {
      final streamUrl = await _resolveStreamUrl(track.id);

      if (_currentRoom?.currentTrack?.id != track.id) {
        return;
      }

      final audioSource = AudioSource.uri(
        Uri.parse(streamUrl),
        tag: MediaItem(
          id: track.id.toString(),
          title: track.title,
          artist: track.artist,
          artUri: track.imageUrl != null ? Uri.parse(track.imageUrl!) : null,
        ),
      );
      await _audioPlayer.setLoopMode(LoopMode.off);
      await _audioPlayer.setAudioSource(audioSource);

      if (_currentRoom != null) {
        _currentRoom!.currentTrack = track;
        _currentRoom!.status = 1;
        _currentRoom!.positionAtLastSync = Duration.zero;
        _currentRoom!.updatedAt = DateTime.now();
        notifyListeners();
      }

      _audioPlayer.play();
    } catch (e, stacktrace) {
      debugPrint('Error playing track: $e\n$stacktrace');
      rethrow;
    }
  }

  void seekTo(Room room, Duration position) {
    if (_currentRoom != null && _currentRoom!.id == room.id) {
      _sendWsEvent(_eventSeekTo, {
        'position': position.inMilliseconds,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    }
  }

  void skipNext() {
    _sendWsEvent(_eventNextTrack, {
      'timestamp': DateTime.now().toUtc().toIso8601String(),
    });
  }

  void skipPrev() {
    _sendWsEvent(_eventSeekTo, {
      'position': 0,
      'timestamp': DateTime.now().toUtc().toIso8601String(),
    });
  }

  Future<void> togglePrivacy(Room room) async {
    try {
      if (!room.isPublic) {
        await ApiClient.post('/rooms/${room.id}/publish');
      } else {
        await ApiClient.post('/rooms/${room.id}/privatize');
      }
      room.isPublic = !room.isPublic;
      notifyListeners();

      unawaited(refreshRooms());
    } catch (e) {
      debugPrint('Failed to toggle privacy: $e');
      notifyListeners();
      rethrow;
    }
  }

  Future<void> toggleLicense(Room room) async {
    try {
      if (!room.isLicensed) {
        await ApiClient.post('/rooms/${room.id}/enable-license');
      } else {
        await ApiClient.post('/rooms/${room.id}/disable-license');
      }
      room.isLicensed = !room.isLicensed;
      notifyListeners();
    } catch (e) {
      debugPrint('Failed to toggle license: $e');
      notifyListeners();
      rethrow;
    }
  }

  void kickListener(Room room, RoomUser listener) {}

  Future<void> promoteToOwner(Room room, RoomUser listener) async {
    try {
      await ApiClient.post(
        '/rooms/${room.id}/transfer-ownership',
        body: {'new_owner_id': listener.id},
      );
    } catch (e) {
      debugPrint('Failed to transfer ownership: $e');
    }
  }
}
