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

  static const String _eventSyncUsers = 'SyncUsers';
  static const String _eventUserJoin = 'UserJoin';
  static const String _eventUserLeave = 'UserLeave';
  static const String _eventRoomClosed = 'RoomClosed';
  static const String _eventPlay = 'Play';
  static const String _eventPause = 'Pause';
  static const String _eventSeekTo = 'SeekTo';
  static const String _eventNextTrack = 'NextTrack';
  static const String _eventQueueAdd = 'QueueAdd';
  static const String _eventQueueRemove = 'QueueRemove';
  static const String _eventQueueReorder = 'QueueReorder';

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
      final queueList = room.queue.toList();
      queueList.sort((a, b) => a.position.compareTo(b.position));
      final nextItem = queueList.first;

      playTrack(nextItem.track);
      removeQueueItem(room, nextItem);
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
      await refreshQueue(_currentRoom!.id);
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
    await refreshQueue(room.id);
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
      case _eventSyncUsers:
        _syncUsers(payload);
        break;
      case _eventUserJoin:
        _addListener(payload);
        break;
      case _eventUserLeave:
        _removeListener(payload);
        break;
      case _eventRoomClosed:
        leaveRoom();
        return;
    }

    if (_isPlaybackEvent(type)) {
      _updateRoomFromPlaybackEvent(type, payload);
      // Refresh queue only for NextTrack, otherwise just update playback state
      if (type == _eventNextTrack) {
        unawaited(refreshQueue(_currentRoom!.id));
      }
    }

    if (_isQueueEvent(type)) {
      unawaited(refreshQueue(_currentRoom!.id));
    }

    notifyListeners();
  }

  void _syncUsers(Map<String, dynamic> payload) {
    final users = payload['users'];
    if (users is! List) return;

    _currentRoom!.listeners
      ..clear()
      ..addAll(
        users.map((user) => _roomUserFromPayload(user)).whereType<RoomUser>(),
      );
  }

  void _addListener(Map<String, dynamic> payload) {
    final listener = _roomUserFromPayload(payload);
    if (listener == null) return;

    final existingIndex = _currentRoom!.listeners.indexWhere(
      (user) => user.id == listener.id,
    );
    if (existingIndex == -1) {
      _currentRoom!.listeners.add(listener);
    }
  }

  void _removeListener(Map<String, dynamic> payload) {
    final id = payload['user_id']?.toString();
    if (id == null) return;

    _currentRoom!.listeners.removeWhere((listener) => listener.id == id);
  }

  RoomUser? _roomUserFromPayload(dynamic payload) {
    if (payload is! Map) return null;

    final id = payload['user_id']?.toString();
    final username = payload['username']?.toString();

    if (id == null || username == null) return null;
    return RoomUser(id: id, username: username);
  }

  void _updateRoomFromPlaybackEvent(
    String? type,
    Map<String, dynamic> payload,
  ) {
    if (_currentRoom == null) return;

    switch (type) {
      case _eventPlay:
        _currentRoom!.status = 1; // 1 = playing
        _currentRoom!.updatedAt = DateTime.now();
        // Sync audio player state for listeners (owner already called play locally)
        if (!_audioPlayer.playing) {
          unawaited(_audioPlayer.play());
        }
        break;
      case _eventPause:
        _currentRoom!.status = 0; // 0 = paused
        final position = payload['position'];
        if (position is int) {
          _currentRoom!.positionAtLastSync = Duration(milliseconds: position);
        }
        _currentRoom!.updatedAt = DateTime.now();
        // Sync audio player state for listeners (owner already called pause locally)
        if (_audioPlayer.playing) {
          unawaited(_audioPlayer.pause());
        }
        break;
      case _eventSeekTo:
        final position = payload['position'];
        if (position is int) {
          _currentRoom!.positionAtLastSync = Duration(milliseconds: position);
        }
        _currentRoom!.updatedAt = DateTime.now();
        break;
      default:
        break;
    }
  }

  bool _isPlaybackEvent(String? type) {
    return const {
      _eventPlay,
      _eventPause,
      _eventSeekTo,
      _eventNextTrack,
    }.contains(type);
  }

  bool _isQueueEvent(String? type) {
    return const {
      _eventQueueAdd,
      _eventQueueRemove,
      _eventQueueReorder,
      _eventNextTrack,
    }.contains(type);
  }

  Future<void> refreshQueue(String roomId) async {
    try {
      final response = await ApiClient.get('/rooms/$roomId/queue');
      if (response is List) {
        if (_currentRoom != null && _currentRoom!.id == roomId) {
          _currentRoom!.queue = response
              .map((data) => QueueItem.fromJson(data))
              .toList();

          _checkAutoPlayNext();
          notifyListeners();
        }
      }
    } catch (e) {
      debugPrint('Failed to refresh queue: $e');
    }
  }

  void _checkAutoPlayNext() {
    if (_currentRoom == null || _currentRoom!.queue.isEmpty) return;

    final sequence = _audioPlayer.audioSource?.sequence;
    final currentTag = sequence != null && sequence.isNotEmpty
        ? sequence.first.tag
        : null;
    final isSilence =
        currentTag is MediaItem && currentTag.id == _silenceTrackId;

    if (isSilence || _audioPlayer.processingState == ProcessingState.idle) {
      _playNextInQueue();
    }
  }

  Future<String> _resolveStreamUrl(int trackId) async {
    final response = await ApiClient.get('/hifi/track/$trackId/stream-url');
    final streamUrl = response['stream_url'] as String?;

    if (streamUrl == null || streamUrl.isEmpty) {
      throw Exception('No stream URL found for track $trackId');
    }

    return streamUrl;
  }

  // Queue management
  Future<void> addTrack(Room room, Track track) async {
    try {
      await ApiClient.post(
        '/rooms/${room.id}/queue',
        body: {'track_id': track.id},
      );
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to add track: $e');
    }
  }

  Future<void> removeQueueItem(Room room, QueueItem item) async {
    try {
      await ApiClient.delete('/rooms/${room.id}/queue', body: {'id': item.id});
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to remove track: $e');
    }
  }

  Future<void> moveQueueItem(
    Room room,
    QueueItem item,
    double newPosition,
  ) async {
    try {
      await ApiClient.patch(
        '/rooms/${room.id}/queue',
        body: {'id': item.id, 'new_position': newPosition},
      );
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to reorder track: $e');
    }
  }

  // Music control
  void togglePlay(Room room) {
    if (_audioPlayer.playing) {
      _audioPlayer.pause();
      _sendWsEvent(_eventPause, {
        'position': _audioPlayer.position.inMilliseconds,
      });
    } else {
      _audioPlayer.play();
      _sendWsEvent(_eventPlay, {
        'position': _audioPlayer.position.inMilliseconds,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    }
  }

  Future<void> playTrack(Track track) async {
    try {
      final streamUrl = await _resolveStreamUrl(track.id);
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
    _audioPlayer.seek(position);
    if (_currentRoom != null && _currentRoom!.id == room.id) {
      _currentRoom!.positionAtLastSync = position;
      _currentRoom!.updatedAt = DateTime.now();
      _sendWsEvent(_eventSeekTo, {
        'position': position.inMilliseconds,
        'timestamp': DateTime.now().toUtc().toIso8601String(),
      });
    }
  }

  void skipNext() {
    _playNextInQueue();
    _sendWsEvent(_eventNextTrack, {
      'timestamp': DateTime.now().toUtc().toIso8601String(),
    });
  }

  void skipPrev() {
    _audioPlayer.seek(Duration.zero);
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
