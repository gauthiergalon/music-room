import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:just_audio/just_audio.dart';

import '../core/logger/logger.dart';
import '../models/queue_item.dart';
import '../models/room.dart';
import '../models/room_user.dart';
import '../models/track.dart';
import '../services/audio_service.dart';
import '../services/room_repository.dart';
import '../services/websocket_service.dart';

class RoomController extends ChangeNotifier with WidgetsBindingObserver {
  static const String _eventRoomState = 'RoomState';
  static const String _eventUserState = 'UserState';
  static const String _eventRoomClosed = 'RoomClosed';
  static const String _eventPlay = 'Play';
  static const String _eventPause = 'Pause';
  static const String _eventSeekTo = 'SeekTo';
  static const String _eventNextTrack = 'NextTrack';

  final AudioService _audioService;
  final WebSocketService _wsService;
  final RoomRepository _roomRepository;

  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;

  String? _roomClosedMessage;
  String? get roomClosedMessage => _roomClosedMessage;
  Track? get currentTrack => _currentRoom?.currentTrack;
  bool get isPlaying => _audioService.isPlaying;
  Duration get playbackPosition => _audioService.position;
  Duration? get playbackDuration => _audioService.duration;

  Stream<Duration> get positionStream => _audioService.positionStream;
  Stream<Duration?> get durationStream => _audioService.durationStream;

  List<Room> _availableRooms = [];
  List<Room> get availableRooms => _availableRooms;

  bool _isInitialRoomState = false;
  StreamSubscription? _playerSubscription;

  RoomController({
    AudioService? audioService,
    WebSocketService? wsService,
    RoomRepository? roomRepository,
  }) : _audioService = audioService ?? AudioService(),
       _wsService = wsService ?? WebSocketService(),
       _roomRepository = roomRepository ?? RoomRepository() {
    WidgetsBinding.instance.addObserver(this);
    _playerSubscription = _audioService.playerStateStream.listen(
      _onPlayerState,
    );
    _wsService.setEventCallback(_handleWsEvent);
  }

  void _onPlayerState(PlayerState state) {
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
      _audioService.stop();
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      refreshRooms();
      if (_currentRoom != null && !_wsService.isConnected) {
        _wsService.connect(_currentRoom!.id);
      }
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _playerSubscription?.cancel();
    _wsService.dispose();
    _audioService.dispose();
    super.dispose();
  }

  Future<void> refreshRooms() async {
    try {
      _availableRooms = await _roomRepository.getRooms();
      notifyListeners();
    } catch (e) {
      logger.error('Failed to refresh rooms', error: e);
      rethrow;
    }
  }

  Future<Room> createRoom() async {
    try {
      final newRoom = await _roomRepository.createRoom();
      _availableRooms.add(newRoom);
      notifyListeners();
      await openRoom(newRoom);
      return newRoom;
    } catch (e) {
      logger.error('Failed to create room', error: e);
      rethrow;
    }
  }

  Future<void> openRoom(Room room) async {
    _currentRoom = room;
    await _wsService.connect(room.id);
    _isInitialRoomState = true;
    notifyListeners();
  }

  void leaveRoom() {
    _audioService.stop();
    _currentRoom = null;
    _wsService.disconnect();
    notifyListeners();
    unawaited(refreshRooms());
  }

  void clearRoomClosedMessage() {
    _roomClosedMessage = null;
    notifyListeners();
  }

  void _sendWsEvent(String eventType, Map<String, dynamic> payload) {
    _wsService.send(eventType, payload);
  }

  void _handleWsEvent(String type, Map<String, dynamic> payload) {
    if (_currentRoom == null) return;

    switch (type) {
      case _eventRoomState:
        _handleRoomState(payload);
        break;
      case _eventUserState:
        _handleUserState(payload);
        break;
      case _eventRoomClosed:
        _roomClosedMessage = 'Room closed by owner';
        leaveRoom();
        return;
    }

    notifyListeners();
  }

  void _handleRoomState(Map<String, dynamic> payload) {
    if (payload['queue'] is List) {
      final queueList = payload['queue'] as List;
      _currentRoom!.queue = queueList.asMap().entries.map<QueueItem>((entry) {
        final queuedTrackJson = entry.value as Map<String, dynamic>;
        final trackJson = queuedTrackJson['track'] as Map<String, dynamic>;
        final position =
            (queuedTrackJson['position'] as num?)?.toDouble() ??
            entry.key.toDouble();

        Track.fromJson(trackJson);

        return QueueItem(
          id: queuedTrackJson['id'] as String,
          roomId: _currentRoom!.id,
          trackId: trackJson['id'] as int,
          position: position,
        );
      }).toList();
    }

    if (payload['current_track'] != null) {
      final track = Track.fromJson(payload['current_track']);
      final bool trackChanged =
          currentTrack?.id != track.id ||
          _isInitialRoomState ||
          _audioService.player.audioSource == null;

      _currentRoom!.currentTrack = track;

      if (trackChanged) {
        final serverTimestamp = DateTime.parse(
          payload['timestamp'] as String,
        ).toUtc();
        final currentPositionMs = (payload['current_position'] as int?) ?? 0;

        _playTrack(track, currentPositionMs, serverTimestamp);
      }
    } else {
      _currentRoom!.currentTrack = null;
      _audioService.stop();
    }

    final isPlaying = payload['is_playing'] == true;
    _currentRoom!.status = isPlaying ? 1 : 0;

    if (isPlaying && !_audioService.isPlaying) {
      _audioService.play();
    } else if (!isPlaying && _audioService.isPlaying) {
      _audioService.pause();
    }

    if (payload['current_position'] != null) {
      int posMs = payload['current_position'] as int;
      if (isPlaying && payload['timestamp'] != null) {
        final playedAt = DateTime.parse(
          payload['timestamp'].toString(),
        ).toUtc();
        final diff = DateTime.now().toUtc().difference(playedAt).inMilliseconds;
        if (diff > 0) posMs += diff;
      }

      final currentPosMs = _audioService.position.inMilliseconds;
      final seekDiff = (posMs - currentPosMs).abs();
      if (seekDiff > 1000) {
        _audioService.seek(Duration(milliseconds: posMs));
      }
    }

    _isInitialRoomState = false;
  }

  void _handleUserState(Map<String, dynamic> payload) {
    final users = payload['user_list'];
    if (users is! List) return;

    _currentRoom!.listeners
      ..clear()
      ..addAll(users.map((u) => _roomUserFromPayload(u)).whereType<RoomUser>());

    final ownerId = payload['owner']?.toString();
    if (ownerId != null) _currentRoom!.owner = ownerId;
  }

  RoomUser? _roomUserFromPayload(dynamic payload) {
    if (payload is! Map) return null;
    final id = payload['user_id']?.toString();
    final username = payload['username']?.toString();
    if (id == null || username == null) return null;
    return RoomUser(id: id, username: username);
  }

  Future<void> _playTrack(
    Track track,
    int serverPositionMs,
    DateTime serverTimestamp,
  ) async {
    try {
      final loadStartTime = DateTime.now().toUtc();
      final streamUrl = await _roomRepository.getStreamUrl(track.id);
      final updatedTrack = Track(
        id: track.id,
        title: track.title,
        artist: track.artist,
        imageUrl: track.imageUrl,
        duration: track.duration,
        streamUrl: streamUrl,
      );

      _currentRoom?.currentTrack = track;
      _currentRoom?.status = 1;
      _currentRoom?.positionAtLastSync = Duration.zero;
      _currentRoom?.updatedAt = DateTime.now();

      final timeSinceServerTimestamp = loadStartTime.difference(
        serverTimestamp,
      );
      final position = Duration(
        milliseconds:
            serverPositionMs + timeSinceServerTimestamp.inMilliseconds,
      );

      await _audioService.playTrack(updatedTrack, position);
    } catch (e) {
      logger.error('Error playing track', error: e);
      rethrow;
    }
  }

  Future<void> addTrack(Room room, Track track) async {
    await _roomRepository.addTrack(room.id, track.id);
  }

  Future<void> removeQueueItem(Room room, QueueItem item) async {
    await _roomRepository.removeQueueItem(room.id, item.id);
  }

  Future<void> reorderQueueItem(
    Room room,
    List<QueueItem> queue,
    int oldIndex,
    int newIndex,
  ) async {
    if (newIndex > oldIndex) newIndex -= 1;
    if (oldIndex < 0 || oldIndex >= queue.length) return;

    final item = queue[oldIndex];
    final reorderedQueue = [...queue];
    reorderedQueue.removeAt(oldIndex);
    final insertIndex = newIndex.clamp(0, reorderedQueue.length);
    reorderedQueue.insert(insertIndex, item);

    List<QueueItem>? previousQueue;
    if (_currentRoom?.id == room.id) {
      previousQueue = _currentRoom!.queue.toList();
      _currentRoom!.queue = reorderedQueue;
      notifyListeners();
    }

    double newPos;
    if (reorderedQueue.length == 1) {
      newPos = 0;
    } else if (insertIndex == 0) {
      newPos = reorderedQueue[1].position - 100;
    } else if (insertIndex == reorderedQueue.length - 1) {
      newPos = reorderedQueue[reorderedQueue.length - 2].position + 100;
    } else {
      newPos =
          (reorderedQueue[insertIndex - 1].position +
              reorderedQueue[insertIndex + 1].position) /
          2;
    }

    try {
      await _roomRepository.reorderQueueItem(room.id, item.id, newPos);
    } catch (e) {
      logger.error('Failed to reorder queue', error: e);
      if (_currentRoom?.id == room.id && previousQueue != null) {
        _currentRoom!.queue = previousQueue;
        notifyListeners();
      }
      rethrow;
    }
  }

  void togglePlay(Room room) {
    if (_audioService.isPlaying) {
      _sendWsEvent(_eventPause, {
        'position': _audioService.position.inMilliseconds,
      });
    } else {
      _sendWsEvent(_eventPlay, {
        'position': _audioService.position.inMilliseconds,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    }
  }

  void seekTo(Room room, Duration position) {
    if (_currentRoom?.id == room.id) {
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
      await _roomRepository.togglePrivacy(room.id, room.isPublic);
      room.isPublic = !room.isPublic;
      notifyListeners();
      unawaited(refreshRooms());
    } catch (e) {
      logger.error('Failed to toggle privacy', error: e);
      notifyListeners();
      rethrow;
    }
  }

  Future<void> toggleLicense(Room room) async {
    try {
      await _roomRepository.toggleLicense(room.id, room.isLicensed);
      room.isLicensed = !room.isLicensed;
      notifyListeners();
    } catch (e) {
      logger.error('Failed to toggle license', error: e);
      notifyListeners();
      rethrow;
    }
  }

  Future<void> promoteToOwner(Room room, RoomUser listener) async {
    try {
      await _roomRepository.transferOwnership(room.id, listener.id);
    } catch (e) {
      logger.error('Failed to transfer ownership', error: e);
    }
  }
}
