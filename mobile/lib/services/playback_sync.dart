import '../models/track.dart';

/// Playback state of a room as last announced by the server.
class RoomPlaybackState {
  final Track? track;
  final bool isPlaying;

  /// Position of the track, in milliseconds, at [timestamp].
  final int positionMs;

  /// Server-clock instant at which [positionMs] was valid.
  final DateTime timestamp;

  const RoomPlaybackState({
    required this.track,
    required this.isPlaying,
    required this.positionMs,
    required this.timestamp,
  });

  RoomPlaybackState copyWith({
    bool? isPlaying,
    int? positionMs,
    DateTime? timestamp,
  }) {
    return RoomPlaybackState(
      track: track,
      isPlaying: isPlaying ?? this.isPlaying,
      positionMs: positionMs ?? this.positionMs,
      timestamp: timestamp ?? this.timestamp,
    );
  }
}

/// What the local audio player is doing right now.
class LocalPlayback {
  /// Track whose audio source is loaded in the player.
  final int? loadedTrackId;

  /// Track whose audio source is currently being fetched/loaded.
  final int? loadingTrackId;
  final bool isPlaying;

  /// The player reached the end of the loaded track.
  final bool isCompleted;
  final Duration position;

  const LocalPlayback({
    this.loadedTrackId,
    this.loadingTrackId,
    this.isPlaying = false,
    this.isCompleted = false,
    this.position = Duration.zero,
  });
}

/// What the player has to do to match the room.
class SyncDecision {
  /// Unload whatever is in the player: the room has no current track.
  final bool stop;

  /// Load the room's current track (then decide again once loaded).
  final bool loadTrack;

  /// `true` to start playing, `false` to pause, `null` to leave as is.
  final bool? setPlaying;

  /// Position to jump to, or `null` when the player is close enough.
  final Duration? seekTo;

  const SyncDecision({
    this.stop = false,
    this.loadTrack = false,
    this.setPlaying,
    this.seekTo,
  });

  static const none = SyncDecision();

  bool get isNone =>
      !stop && !loadTrack && setPlaying == null && seekTo == null;
}

/// Estimates the difference between the server clock and the local clock.
///
/// Every server message carries the server time at which it was sent. The
/// observed `serverTime - localReceiveTime` equals the real offset minus the
/// network latency, so the largest recent observation (the one with the least
/// latency) is the best estimate. Only the last few samples are kept so the
/// estimate follows a local clock that gets adjusted.
class ServerClock {
  ServerClock({this.window = 8});

  final int window;
  final List<Duration> _samples = [];

  void observe(DateTime serverTime, DateTime localReceiveTime) {
    _samples.add(serverTime.toUtc().difference(localReceiveTime.toUtc()));
    if (_samples.length > window) _samples.removeAt(0);
  }

  void reset() => _samples.clear();

  /// `serverClock - localClock`, or zero before the first observation.
  Duration get offset {
    if (_samples.isEmpty) return Duration.zero;
    return _samples.reduce((a, b) => a > b ? a : b);
  }

  /// Current server time for the given local time.
  DateTime now(DateTime localNow) => localNow.toUtc().add(offset);
}

/// A `RoomState` is stale when it is not newer than the last applied one.
bool isStaleVersion(int version, int? lastApplied) =>
    lastApplied != null && version <= lastApplied;

/// Where the track should be at [serverNow].
///
/// Time only advances while the room is playing, and the result never leaves
/// the track.
Duration expectedPosition(RoomPlaybackState state, DateTime serverNow) {
  var ms = state.positionMs;
  if (state.isPlaying) {
    final elapsed = serverNow.difference(state.timestamp).inMilliseconds;
    if (elapsed > 0) ms += elapsed;
  }
  if (ms < 0) ms = 0;
  final durationMs = state.track?.duration.inMilliseconds ?? 0;
  if (durationMs > 0 && ms > durationMs) ms = durationMs;
  return Duration(milliseconds: ms);
}

/// True when the room is still "playing" a track that is already over, i.e.
/// it is waiting for the controller to ask for the next track.
bool isPastEnd(
  RoomPlaybackState state,
  DateTime serverNow, {
  Duration tolerance = const Duration(seconds: 1),
}) {
  final duration = state.track?.duration ?? Duration.zero;
  if (duration <= Duration.zero) return false;
  return expectedPosition(state, serverNow) >= duration - tolerance;
}

/// Compares the room state with the local player and says what to change.
SyncDecision decideSync({
  required RoomPlaybackState target,
  required LocalPlayback local,
  required DateTime serverNow,
  Duration seekThreshold = const Duration(seconds: 1),
}) {
  final track = target.track;
  if (track == null) {
    final hasMedia =
        local.loadedTrackId != null || local.loadingTrackId != null;
    return hasMedia ? const SyncDecision(stop: true) : SyncDecision.none;
  }

  if (local.loadedTrackId != track.id) {
    if (local.loadingTrackId == track.id) return SyncDecision.none;
    return const SyncDecision(loadTrack: true);
  }

  final expected = expectedPosition(target, serverNow);
  final finishedLocally =
      local.isCompleted &&
      isPastEnd(target, serverNow, tolerance: seekThreshold);

  Duration? seekTo;
  if (!finishedLocally && (expected - local.position).abs() > seekThreshold) {
    seekTo = expected;
  }

  final setPlaying = target.isPlaying != local.isPlaying
      ? target.isPlaying
      : null;

  if (seekTo == null && setPlaying == null) return SyncDecision.none;
  return SyncDecision(seekTo: seekTo, setPlaying: setPlaying);
}
