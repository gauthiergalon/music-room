import 'dart:async';

import 'package:flutter/widgets.dart';

import '../core/exceptions/api_exception.dart';
import '../core/logger/logger.dart';
import '../models/queue_item.dart';
import '../models/room.dart';
import '../models/room_user.dart';
import '../models/track.dart';
import '../services/audio_service.dart';
import '../services/playback_sync.dart';
import '../services/queue_order.dart';
import '../services/room_repository.dart';
import '../services/websocket_service.dart';

class RoomController extends ChangeNotifier with WidgetsBindingObserver {
  static const String _eventWelcome = 'Welcome';
  static const String _eventRoomState = 'RoomState';
  static const String _eventUserState = 'UserState';
  static const String _eventRoomClosed = 'RoomClosed';
  static const String _eventError = 'Error';
  static const String _eventPlay = 'Play';
  static const String _eventPause = 'Pause';
  static const String _eventSeekTo = 'SeekTo';
  static const String _eventNextTrack = 'NextTrack';

  /// How often the player position is compared with the room's.
  static const Duration driftCheckInterval = Duration(seconds: 5);

  /// After a local play/pause/seek, server corrections for the same track
  /// are held back this long: the answers to our own commands arrive one by
  /// one and would otherwise drag the player back and forth.
  static const Duration localActionGrace = Duration(milliseconds: 1500);

  final PlaybackEngine _audio;
  final WebSocketService _ws;
  final RoomRepository _roomRepository;
  final DateTime Function() _now;

  RoomController({
    required PlaybackEngine audioService,
    WebSocketService? wsService,
    RoomRepository? roomRepository,
    DateTime Function()? now,
  }) : _audio = audioService,
       _ws = wsService ?? WebSocketService(),
       _roomRepository = roomRepository ?? RoomRepository(),
       _now = now ?? DateTime.now {
    WidgetsBinding.instance.addObserver(this);
    _completedSubscription = _audio.completedStream.listen(
      (_) => _onPlaybackCompleted(),
    );
    _ws.onEvent = _handleWsEvent;
    _ws.onRoomUnavailable = _onRoomUnavailable;
    _ws.roomProbe = _roomRepository.isRoomAvailable;
    _ws.state.addListener(_onConnectionStateChanged);
  }

  // ---------------------------------------------------------------- state

  Room? _currentRoom;
  List<Room> _availableRooms = const [];
  String? _currentUserId;
  String? _connectionId;

  String? _roomClosedMessage;
  String? _playerErrorMessage;

  final ServerClock _clock = ServerClock();
  RoomPlaybackState? _target;
  int? _lastVersion;

  /// Incremented for every track load; a load that finishes with an older
  /// value has been superseded and must not touch the player.
  int _loadGeneration = 0;
  int? _loadedTrackId;
  int? _loadingTrackId;

  /// Identifies the playback (queue entry + start instant) NextTrack was last
  /// requested for, so it is requested once.
  String? _nextTrackRequestedFor;

  DateTime? _localActionUntil;
  Timer? _reconcileTimer;
  Timer? _driftTimer;
  StreamSubscription<void>? _completedSubscription;
  bool _disposed = false;

  Room? get currentRoom => _currentRoom;
  List<Room> get availableRooms => _availableRooms;
  Track? get currentTrack => _currentRoom?.currentTrack;
  List<QueueItem> get queue => _currentRoom?.queue ?? const [];

  /// Whether the room (not necessarily this device yet) is playing.
  bool get isPlaying => _currentRoom?.isPlaying ?? false;

  WsConnectionState get connectionState => _ws.state.value;

  Duration get playbackPosition => _audio.position;
  Stream<Duration> get positionStream => _audio.positionStream;
  Stream<Duration?> get durationStream => _audio.durationStream;

  bool get hasRoomClosedMessage => _roomClosedMessage != null;
  bool get hasPlayerErrorMessage => _playerErrorMessage != null;

  /// Whether this device may control the playback of the current room: the
  /// delegated device if there is one, otherwise any device of the owner.
  bool get canControl {
    final room = _currentRoom;
    final me = _currentUserId;
    if (room == null || me == null) return false;
    if (connectionState != WsConnectionState.connected) return false;

    final delegateUserId = room.delegateUserId;
    if (delegateUserId != null && delegateUserId.isNotEmpty) {
      return delegateUserId == me &&
          _connectionId != null &&
          room.delegateConnectionId == _connectionId;
    }
    return room.owner == me;
  }

  /// Sets the logged-in user. Does not notify: it is called while the widget
  /// tree is being built, and only changes while no room is open.
  void setCurrentUserId(String? userId) {
    _currentUserId = userId;
  }

  /// Returns the pending "room closed" message, if any, and clears it.
  String? takeRoomClosedMessage() {
    final message = _roomClosedMessage;
    _roomClosedMessage = null;
    return message;
  }

  /// Returns the pending player error message, if any, and clears it.
  String? takePlayerErrorMessage() {
    final message = _playerErrorMessage;
    _playerErrorMessage = null;
    return message;
  }

  // ------------------------------------------------------------ lifecycle

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state != AppLifecycleState.resumed) return;
    _ws.reconnectNow();
    unawaited(_refreshRoomsQuietly());
  }

  @override
  void dispose() {
    _disposed = true;
    WidgetsBinding.instance.removeObserver(this);
    _completedSubscription?.cancel();
    _driftTimer?.cancel();
    _reconcileTimer?.cancel();
    _ws.state.removeListener(_onConnectionStateChanged);
    _ws.dispose();
    super.dispose();
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  // ---------------------------------------------------------------- rooms

  Future<void> refreshRooms() async {
    try {
      _availableRooms = await _roomRepository.getRooms();
      _notify();
    } catch (e) {
      logger.error('Failed to refresh rooms', error: e);
      rethrow;
    }
  }

  Future<void> _refreshRoomsQuietly() async {
    try {
      await refreshRooms();
    } catch (_) {
      // Already logged; the list simply stays as it was.
    }
  }

  Future<Room> createRoom() async {
    try {
      final newRoom = await _roomRepository.createRoom();
      _availableRooms = [..._availableRooms, newRoom];
      _notify();
      await openRoom(newRoom);
      return newRoom;
    } catch (e) {
      logger.error('Failed to create room', error: e);
      rethrow;
    }
  }

  Future<void> openRoom(Room room) async {
    // `connect` below replaces any previous connection itself.
    _closeRoomSession(disconnect: false);
    _currentRoom = room;
    _driftTimer = Timer.periodic(driftCheckInterval, (_) => _checkDrift());
    _notify();
    await _ws.connect(room.id);
  }

  void leaveRoom() {
    if (_currentRoom == null) return;
    _closeRoomSession();
    _notify();
    unawaited(_refreshRoomsQuietly());
  }

  /// Leaves the room and forgets everything about the user (logout).
  void reset() {
    _closeRoomSession();
    _availableRooms = const [];
    _roomClosedMessage = null;
    _playerErrorMessage = null;
    _notify();
  }

  void _closeRoomSession({bool disconnect = true}) {
    _driftTimer?.cancel();
    _driftTimer = null;
    _reconcileTimer?.cancel();
    _reconcileTimer = null;
    _localActionUntil = null;

    _loadGeneration++;
    _loadedTrackId = null;
    _loadingTrackId = null;
    _nextTrackRequestedFor = null;
    _target = null;
    _lastVersion = null;
    _connectionId = null;
    _clock.reset();

    if (_currentRoom != null) {
      _currentRoom = null;
      unawaited(_audio.stop());
      if (disconnect) unawaited(_ws.disconnect());
    }
  }

  void _onRoomUnavailable() {
    if (_currentRoom == null) return;
    _roomClosedMessage = 'This room is no longer available';
    leaveRoom();
  }

  void _onConnectionStateChanged() {
    if (connectionState == WsConnectionState.connected) {
      // Versions restart with every connection.
      _lastVersion = null;
    }
    _notify();
  }

  // ------------------------------------------------------------ WS events

  void _handleWsEvent(String type, Map<String, dynamic> payload) {
    if (_currentRoom == null) return;

    switch (type) {
      case _eventWelcome:
        _handleWelcome(payload);
      case _eventRoomState:
        _handleRoomState(payload);
      case _eventUserState:
        _handleUserState(payload);
      case _eventRoomClosed:
        _roomClosedMessage = 'Room closed by owner';
        leaveRoom();
        return;
      case _eventError:
        final message = payload['message']?.toString();
        if (message != null && message.isNotEmpty) {
          _playerErrorMessage = message;
        }
      default:
        return;
    }

    _notify();
  }

  void _handleWelcome(Map<String, dynamic> payload) {
    _connectionId = payload['connection_id']?.toString();
    _lastVersion = null;
    _observeServerTime(payload['server_time'], _now());
  }

  void _handleRoomState(Map<String, dynamic> payload) {
    final room = _currentRoom!;
    final receivedAt = _now();
    _observeServerTime(payload['server_time'], receivedAt);

    final version = (payload['version'] as num?)?.toInt();
    if (version != null) {
      if (isStaleVersion(version, _lastVersion)) return;
      _lastVersion = version;
    }

    final rawQueue = payload['queue'];
    if (rawQueue is List) {
      room.queue = [
        for (final (index, entry) in rawQueue.indexed)
          QueueItem.fromJson(
            Map<String, dynamic>.from(entry as Map),
            fallback: index.toDouble(),
          ),
      ];
    }

    final rawTrack = payload['current_track'];
    final track = rawTrack is Map
        ? Track.fromJson(Map<String, dynamic>.from(rawTrack))
        : null;
    final isPlaying = track != null && payload['is_playing'] == true;

    room.currentTrack = track;
    room.isPlaying = isPlaying;

    _target = RoomPlaybackState(
      track: track,
      itemId: track == null ? null : payload['current_item_id']?.toString(),
      isPlaying: isPlaying,
      positionMs: (payload['current_position'] as num?)?.toInt() ?? 0,
      timestamp: _parseTime(payload['timestamp']) ?? _clock.now(receivedAt),
    );

    // While our own commands are still being answered, only a change of
    // track is applied right away; the rest is reconciled once they settle.
    final sameTrack = track != null && track.id == _loadedTrackId;
    if (!(sameTrack && _inLocalActionGrace(receivedAt))) {
      _applyTarget();
    }
  }

  void _handleUserState(Map<String, dynamic> payload) {
    final room = _currentRoom!;
    final users = payload['user_list'];
    if (users is! List) return;

    room.listeners = users
        .map(_roomUserFromPayload)
        .whereType<RoomUser>()
        .toList();

    final ownerId = payload['owner']?.toString();
    if (ownerId != null) room.owner = ownerId;
    room.delegateUserId = payload['delegate_user_id']?.toString();
    room.delegateConnectionId = payload['delegate_connection_id']?.toString();
  }

  RoomUser? _roomUserFromPayload(dynamic payload) {
    if (payload is! Map) return null;
    final id = payload['user_id']?.toString();
    final username = payload['username']?.toString();
    if (id == null || username == null) return null;
    return RoomUser(
      id: id,
      username: username,
      connectionId: payload['connection_id']?.toString() ?? '',
      deviceName: payload['device_name']?.toString() ?? 'Unknown device',
    );
  }

  void _observeServerTime(dynamic raw, DateTime receivedAt) {
    final serverTime = _parseTime(raw);
    if (serverTime != null) _clock.observe(serverTime, receivedAt);
  }

  static DateTime? _parseTime(dynamic raw) =>
      raw is String ? DateTime.tryParse(raw)?.toUtc() : null;

  // ----------------------------------------------------------------- sync

  bool _inLocalActionGrace(DateTime now) {
    final until = _localActionUntil;
    return until != null && now.isBefore(until);
  }

  void _noteLocalAction() {
    _localActionUntil = _now().add(localActionGrace);
    _reconcileTimer?.cancel();
    _reconcileTimer = Timer(localActionGrace, () {
      _localActionUntil = null;
      _applyTarget();
    });
  }

  void _checkDrift() {
    final target = _target;
    if (target == null || _loadedTrackId == null) return;
    if (target.track?.id != _loadedTrackId) return;
    if (_inLocalActionGrace(_now())) return;
    _applyTarget();
  }

  /// Brings the local player in line with the latest room state.
  void _applyTarget() {
    final target = _target;
    if (target == null || _currentRoom == null) return;

    final serverNow = _clock.now(_now());
    final decision = decideSync(
      target: target,
      local: LocalPlayback(
        loadedTrackId: _loadedTrackId,
        loadingTrackId: _loadingTrackId,
        isPlaying: _audio.isPlaying,
        isCompleted: _audio.isCompleted,
        position: _audio.position,
      ),
      serverNow: serverNow,
    );

    if (decision.stop) {
      _loadGeneration++;
      _loadedTrackId = null;
      _loadingTrackId = null;
      unawaited(_audio.stop());
      return;
    }

    if (decision.loadTrack) {
      unawaited(_loadTrack(target.track!));
      return;
    }

    final seekTo = decision.seekTo;
    if (seekTo != null) unawaited(_audio.seek(seekTo));

    final setPlaying = decision.setPlaying;
    if (setPlaying == true) {
      _audio.play();
    } else if (setPlaying == false) {
      _audio.pause();
    }

    // The room is waiting for the next track and this player is already at
    // the end (e.g. it finished while the connection was down).
    if (_audio.isCompleted &&
        target.isPlaying &&
        isPastEnd(target, serverNow)) {
      _requestNextTrackOnce();
    }
  }

  Future<void> _loadTrack(Track track) async {
    final generation = ++_loadGeneration;
    _loadingTrackId = track.id;
    _loadedTrackId = null;
    // Don't keep playing the previous track while the new one is fetched.
    if (_audio.isPlaying) _audio.pause();

    try {
      final streamUrl = await _roomRepository.getStreamUrl(track.id);
      if (generation != _loadGeneration) return;

      // The room kept moving while the URL was fetched: start from where it
      // is now, not from where it was when the load began.
      final target = _target;
      final initialPosition = target != null && target.track?.id == track.id
          ? expectedPosition(target, _clock.now(_now()))
          : Duration.zero;

      await _audio.load(track, streamUrl, initialPosition: initialPosition);
      if (generation != _loadGeneration) return;

      _loadedTrackId = track.id;
      _loadingTrackId = null;
      // Loading took time too: decide again against the current room state
      // (play or stay paused, and catch up if needed).
      _applyTarget();
    } catch (e) {
      if (generation != _loadGeneration) return;
      _loadingTrackId = null;
      logger.error('Error loading track', error: e);
      _playerErrorMessage = _loadErrorMessage(e);
      _notify();
    }
  }

  /// What to tell the user when the current track cannot be loaded.
  static String _loadErrorMessage(Object error) {
    if (error is! ApiException) return 'Error playing track.';
    final status = error.statusCode ?? 0;
    // A 4xx answer is the server explaining why (for instance "This track is
    // not available for streaming"); anything else is a technical failure.
    return status >= 400 && status < 500
        ? error.message
        : 'Player error: unable to load the track.';
  }

  void _onPlaybackCompleted() {
    if (_currentRoom == null || _loadedTrackId == null) return;
    _requestNextTrackOnce();
  }

  /// Asks the server for the next track, once per playback, and only from a
  /// device that controls the room.
  void _requestNextTrackOnce() {
    final trackId = _loadedTrackId;
    final target = _target;
    if (trackId == null || target == null || !canControl) return;
    // The room already moved to another track: the server would ignore it.
    if (target.track?.id != trackId) return;

    final itemId = target.itemId;
    final key = '$itemId@${target.timestamp.toIso8601String()}';
    if (_nextTrackRequestedFor == key) return;
    _nextTrackRequestedFor = key;

    _ws.send(_eventNextTrack, {'item_id': itemId});
  }

  /// Position to report with a play/pause command.
  int _currentPositionMs() {
    final target = _target;
    final track = currentTrack;
    if (track != null && _loadedTrackId == track.id) {
      return _audio.position.inMilliseconds;
    }
    if (target == null) return 0;
    return expectedPosition(target, _clock.now(_now())).inMilliseconds;
  }

  // ------------------------------------------------------------- controls

  void togglePlay() {
    final room = _currentRoom;
    final target = _target;
    if (room == null || target == null || room.currentTrack == null) return;
    if (!canControl) return;

    final positionMs = _currentPositionMs();
    final loaded = _loadedTrackId == room.currentTrack!.id;

    if (room.isPlaying) {
      if (loaded) _audio.pause();
      room.isPlaying = false;
      _target = target.copyWith(isPlaying: false, positionMs: positionMs);
      _ws.send(_eventPause, {'position': positionMs});
    } else {
      if (loaded) _audio.play();
      room.isPlaying = true;
      _target = target.copyWith(
        isPlaying: true,
        positionMs: positionMs,
        timestamp: _clock.now(_now()),
      );
      _ws.send(_eventPlay, {'position': positionMs});
    }

    _noteLocalAction();
    _notify();
  }

  void seekTo(Duration position) {
    final room = _currentRoom;
    final target = _target;
    if (room == null || target == null || room.currentTrack == null) return;
    if (!canControl) return;

    if (_loadedTrackId == room.currentTrack!.id) {
      unawaited(_audio.seek(position));
    }
    _target = target.copyWith(
      positionMs: position.inMilliseconds,
      timestamp: _clock.now(_now()),
    );
    _ws.send(_eventSeekTo, {'position': position.inMilliseconds});
    _noteLocalAction();
  }

  void skipNext() {
    if (!canControl) return;
    _ws.send(_eventNextTrack, {'item_id': _target?.itemId});
  }

  void skipPrev() => seekTo(Duration.zero);

  // ---------------------------------------------------------------- queue

  Future<void> addTrack(Track track) async {
    final room = _currentRoom;
    if (room == null) return;
    await _roomRepository.addTrack(room.id, track.id);
  }

  Future<void> removeQueueItem(QueueItem item) async {
    final room = _currentRoom;
    if (room == null) return;

    // Removed locally right away: the item's widget has already been
    // dismissed and must not come back before the server confirms.
    final previousQueue = room.queue;
    room.queue = previousQueue.where((q) => q.id != item.id).toList();
    _notify();

    try {
      await _roomRepository.removeQueueItem(room.id, item.id);
    } on ApiException catch (e) {
      if (e.statusCode == 404) return; // Already gone.
      _restoreQueue(room, previousQueue);
      rethrow;
    } catch (e) {
      _restoreQueue(room, previousQueue);
      rethrow;
    }
  }

  Future<void> reorderQueueItem(int oldIndex, int newIndex) async {
    final room = _currentRoom;
    if (room == null) return;

    final previousQueue = room.queue;
    if (oldIndex < 0 || oldIndex >= previousQueue.length) return;

    final item = previousQueue[oldIndex];
    final reordered = [...previousQueue]..removeAt(oldIndex);
    final insertIndex = newIndex.clamp(0, reordered.length);
    reordered.insert(insertIndex, item);

    room.queue = reordered;
    _notify();

    final newPosition = computeReorderPosition(
      reordered.map((q) => q.position).toList(),
      insertIndex,
    );

    try {
      await _roomRepository.reorderQueueItem(room.id, item.id, newPosition);
    } catch (e) {
      logger.error('Failed to reorder queue', error: e);
      _restoreQueue(room, previousQueue);
      rethrow;
    }
  }

  void _restoreQueue(Room room, List<QueueItem> queue) {
    if (!identical(_currentRoom, room)) return;
    room.queue = queue;
    _notify();
  }

  // ------------------------------------------------------------- settings

  Future<void> delegateDevice(RoomUser user) async {
    final room = _currentRoom;
    if (room == null) return;
    await _roomRepository.delegateDevice(room.id, user.id, user.connectionId);
  }

  Future<void> revokeDelegate() async {
    final room = _currentRoom;
    if (room == null) return;
    await _roomRepository.revokeDelegate(room.id);
  }

  Future<void> inviteToRoom(String inviteeId) async {
    final room = _currentRoom;
    if (room == null) return;
    await _roomRepository.inviteToRoom(room.id, inviteeId);
  }

  Future<void> togglePrivacy() async {
    final room = _currentRoom;
    if (room == null) return;
    try {
      await _roomRepository.togglePrivacy(room.id, room.isPublic);
      room.isPublic = !room.isPublic;
      _notify();
      unawaited(_refreshRoomsQuietly());
    } catch (e) {
      logger.error('Failed to toggle privacy', error: e);
      rethrow;
    }
  }

  Future<void> toggleLicense() async {
    final room = _currentRoom;
    if (room == null) return;
    try {
      await _roomRepository.toggleLicense(room.id, room.isLicensed);
      room.isLicensed = !room.isLicensed;
      _notify();
    } catch (e) {
      logger.error('Failed to toggle license', error: e);
      rethrow;
    }
  }
}
