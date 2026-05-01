import 'dart:convert';
import 'package:flutter/widgets.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/models/queue_item.dart';
import '../models/room.dart';
import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';
import '../models/room_user.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';

class RoomController extends ChangeNotifier with WidgetsBindingObserver {
  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;
  Track? get currentTrack => _currentRoom?.currentTrack;
  bool get isPlaying => _audioPlayer.playing;
  Duration get playbackPosition => _audioPlayer.position;
  Duration? get playbackDuration => _audioPlayer.duration;

  List<Room> _availableRooms = [];
  List<Room> get availableRooms => _availableRooms;

  WebSocketChannel? _wsChannel;
  final AudioPlayer _audioPlayer = AudioPlayer(); // Le lecteur en background

  RoomController() {
    WidgetsBinding.instance.addObserver(this);

    // Écouter les événements du player pour les dispatcher au front
    _audioPlayer.playerStateStream.listen((state) {
      if (state.processingState == ProcessingState.completed) {
        _playNextInQueue();
      }
      notifyListeners();
    });
  }

  void _playNextInQueue() {
    final room = _currentRoom;
    if (room != null && room.queue.isNotEmpty) {
      // Find the first track by position
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
    _audioPlayer.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      // Reconect to WebSocket when app resumes if we have a current room
      if (_currentRoom != null && _wsChannel?.closeCode != null) {
        debugPrint('App resumed, verifying if room still exists...');
        _checkAndReconnect();
      }
    }
  }

  Future<void> _checkAndReconnect() async {
    if (_currentRoom == null) return;
    try {
      // On vérifie si la room existe toujours sur le serveur (GET /rooms/:id)
      await ApiClient.get('/rooms/${_currentRoom!.id}');

      debugPrint('Room exists, reconnecting WS...');
      _connectWebSocket(_currentRoom!.id);
      await refreshQueue(_currentRoom!.id);
    } catch (e) {
      // Si on reçoit une 404, la room a été supprimée suite à la déconnexion
      debugPrint('Room no longer exists, leaving room: $e');
      leaveRoom();
    }
  }

  Future<void> _refreshCurrentRoom(String roomId) async {
    try {
      final response = await ApiClient.get('/rooms/$roomId');
      if (_currentRoom == null || _currentRoom!.id != roomId) return;

      final refreshedRoom = Room.fromJson(response);
      refreshedRoom.queue = _currentRoom!.queue;
      refreshedRoom.listeners = _currentRoom!.listeners;
      _currentRoom = refreshedRoom;
      notifyListeners();
    } catch (e) {
      debugPrint('Failed to refresh current room: $e');
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
    _connectWebSocket(room.id);

    // Lancer une musique 'silencieuse' pour garder la room en vie en arrière-plan sans son
    // tant que l'host n'a pas mis de vraie musique !
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

      // Plays a silent audio file in loop to keep the app awake in the background
      // and maintain the WebSocket connection alive while the queue is empty.
      final silenceSource = AudioSource.asset(
        'assets/audio/silence.mp3',
        tag: const MediaItem(
          id: 'silence_placeholder',
          title: 'Music Room Active',
          artist: 'Waiting for a track...',
        ),
      );

      await _audioPlayer.setLoopMode(LoopMode.one);
      await _audioPlayer.setAudioSource(silenceSource);
      _audioPlayer.play();
      await Future.delayed(const Duration(seconds: 1));
      await _audioPlayer.pause();
    } catch (e) {
      debugPrint('Failed to play silence background: $e');
    }
  }

  void leaveRoom() {
    _audioPlayer.stop(); // Arrêter la musique en quittant
    _currentRoom = null;
    _wsChannel?.sink.close();
    _wsChannel = null;
    notifyListeners();

    Future.delayed(const Duration(milliseconds: 100), () {
      refreshRooms();
    });
  }

  void _connectWebSocket(String roomId) async {
    final token = await ApiClient.getToken();
    if (token == null) return;

    final baseUrl =
        '${ApiClient.baseUrl.replaceFirst('http', 'ws')}/rooms/$roomId/ws';
    _wsChannel = createWsChannel(baseUrl, token);

    try {
      _wsChannel!.stream.listen(
        (message) {
          debugPrint('WS Message: $message');
          final data = jsonDecode(message);
          _handleWsEvent(data);
        },
        onError: (err) {
          debugPrint('WS Error: $err');
        },
        onDone: () {
          debugPrint('WS Closed with code: ${_wsChannel?.closeCode}');
        },
      );
    } catch (e) {
      debugPrint('Error listening to WS: $e');
    }
  }

  void _handleWsEvent(Map<String, dynamic> data) {
    if (_currentRoom == null) return;
    final type = data['type'];
    final payload = data['payload'] ?? {};
    if (type == 'SyncUsers') {
      final users = payload['users'] as List?;
      if (users != null) {
        _currentRoom!.listeners.clear();
        for (var u in users) {
          final id = u['user_id'] as String?;
          final name = u['username'] as String?;
          if (id != null && name != null) {
            _currentRoom!.listeners.add(RoomUser(id: id, username: name));
          }
        }
      }
    } else if (type == 'UserJoin') {
      final id = payload['user_id'] as String?;
      final name = payload['username'] as String?;
      if (id != null && name != null) {
        final existing = _currentRoom!.listeners.indexWhere((u) => u.id == id);
        if (existing == -1) {
          _currentRoom!.listeners.add(RoomUser(id: id, username: name));
        }
      }
    } else if (type == 'UserLeave') {
      final id = payload['user_id'] as String?;
      if (id != null) {
        _currentRoom!.listeners.removeWhere((u) => u.id == id);
      }
    } else if (type == 'RoomClosed') {
      leaveRoom();
      return;
    }

    if (type == 'Play' ||
        type == 'Pause' ||
        type == 'SeekTo' ||
        type == 'NextTrack') {
      _refreshCurrentRoom(_currentRoom!.id);
    }

    // React to events (add simple logs or state updates)
    if (type == 'QueueAdd' || type == 'QueueRemove' || type == 'QueueReorder') {
      refreshQueue(_currentRoom!.id);
    } else if (type == 'NextTrack') {
      refreshQueue(_currentRoom!.id);
    }
    notifyListeners();
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

    final currentTag = _audioPlayer.audioSource?.sequence.first.tag;
    final isSilence =
        currentTag is MediaItem && currentTag.id == 'silence_placeholder';

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
    } else {
      _audioPlayer.play();
    }
    // Should call WS event or endpoint to sync others if owner
  }

  Future<void> playTrack(Track track) async {
    try {
      final streamUrl = await _resolveStreamUrl(track.id);
      debugPrint('Preparing to play: $streamUrl');
      final audioSource = AudioSource.uri(
        Uri.parse(streamUrl),
        tag: MediaItem(
          id: track.id.toString(),
          title: track.title, // 'title' in Track
          artist: track.artist,
          artUri: track.imageUrl != null ? Uri.parse(track.imageUrl!) : null,
        ),
      );
      await _audioPlayer.setLoopMode(LoopMode.off);
      await _audioPlayer.setAudioSource(audioSource);
      debugPrint('Audio source loaded, calling play()');

      if (_currentRoom != null) {
        _currentRoom!.currentTrack = track;
        _currentRoom!.status = 1;
        _currentRoom!.positionAtLastSync = Duration.zero;
        _currentRoom!.updatedAt = DateTime.now();
        notifyListeners();
      }

      _audioPlayer.play();
      debugPrint('Play command sent');
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
    }
    // Should call WS event or endpoint
  }

  void skipNext() {
    _playNextInQueue();
  }

  void skipPrev() {
    _audioPlayer.seek(Duration.zero);
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

      refreshRooms(); // Update available list
    } catch (e) {
      debugPrint('Failed to toggle privacy: $e');
      notifyListeners(); // Keep UI in sync if the switch failed
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
