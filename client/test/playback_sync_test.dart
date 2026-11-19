import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/services/playback_sync.dart';

const _track = Track(
  id: 7,
  title: 'Song',
  artist: 'Artist',
  duration: Duration(minutes: 3),
);

final _t0 = DateTime.utc(2026, 1, 1, 12);

RoomPlaybackState _state({
  Track? track = _track,
  bool isPlaying = true,
  int positionMs = 10000,
  DateTime? timestamp,
}) => RoomPlaybackState(
  track: track,
  isPlaying: isPlaying,
  positionMs: positionMs,
  timestamp: timestamp ?? _t0,
);

void main() {
  group('expectedPosition', () {
    test('a paused room stays where it was paused', () {
      final position = expectedPosition(
        _state(isPlaying: false),
        _t0.add(const Duration(minutes: 1)),
      );
      expect(position, const Duration(seconds: 10));
    });

    test('a playing room advances with the server clock', () {
      final position = expectedPosition(
        _state(),
        _t0.add(const Duration(seconds: 25)),
      );
      expect(position, const Duration(seconds: 35));
    });

    test('never goes past the end of the track', () {
      final position = expectedPosition(
        _state(),
        _t0.add(const Duration(hours: 1)),
      );
      expect(position, _track.duration);
    });

    test('a timestamp in the future does not move the position back', () {
      final position = expectedPosition(
        _state(),
        _t0.subtract(const Duration(seconds: 5)),
      );
      expect(position, const Duration(seconds: 10));
    });
  });

  group('ServerClock', () {
    test('is neutral before any observation', () {
      final clock = ServerClock();
      expect(clock.offset, Duration.zero);
      expect(clock.now(_t0), _t0);
    });

    test('keeps the observation with the least latency', () {
      final clock = ServerClock();
      // Real offset is +2 s; the samples are reduced by 300 ms and 50 ms of
      // network latency.
      clock.observe(_t0.add(const Duration(milliseconds: 1700)), _t0);
      clock.observe(_t0.add(const Duration(milliseconds: 1950)), _t0);
      expect(clock.offset, const Duration(milliseconds: 1950));
      expect(clock.now(_t0), _t0.add(const Duration(milliseconds: 1950)));
    });

    test('forgets old observations', () {
      final clock = ServerClock(window: 2);
      clock.observe(_t0.add(const Duration(seconds: 5)), _t0);
      clock.observe(_t0.add(const Duration(seconds: 1)), _t0);
      clock.observe(_t0.add(const Duration(seconds: 2)), _t0);
      expect(clock.offset, const Duration(seconds: 2));
    });
  });

  group('isStaleVersion', () {
    test('accepts the first and any newer version', () {
      expect(isStaleVersion(1, null), isFalse);
      expect(isStaleVersion(5, 4), isFalse);
    });

    test('rejects an older or repeated version', () {
      expect(isStaleVersion(4, 4), isTrue);
      expect(isStaleVersion(3, 4), isTrue);
    });
  });

  group('decideSync', () {
    test('loads the track when nothing is loaded', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(),
        serverNow: _t0,
      );
      expect(decision.loadTrack, isTrue);
      expect(decision.setPlaying, isNull);
    });

    test('loads the new track when another one is loaded', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(loadedTrackId: 99, isPlaying: true),
        serverNow: _t0,
      );
      expect(decision.loadTrack, isTrue);
    });

    test('waits while the right track is already loading', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(loadingTrackId: 7),
        serverNow: _t0,
      );
      expect(decision.isNone, isTrue);
    });

    test('stops when the room has no track any more', () {
      final decision = decideSync(
        target: _state(track: null, isPlaying: false),
        local: const LocalPlayback(loadedTrackId: 7, isPlaying: true),
        serverNow: _t0,
      );
      expect(decision.stop, isTrue);
    });

    test('does nothing for an empty room and an empty player', () {
      final decision = decideSync(
        target: _state(track: null, isPlaying: false),
        local: const LocalPlayback(),
        serverNow: _t0,
      );
      expect(decision.isNone, isTrue);
    });

    test('a loaded track in a paused room is not started', () {
      final decision = decideSync(
        target: _state(isPlaying: false),
        local: const LocalPlayback(
          loadedTrackId: 7,
          position: Duration(seconds: 10),
        ),
        serverNow: _t0.add(const Duration(minutes: 2)),
      );
      expect(decision.isNone, isTrue);
    });

    test('pauses a player that is still playing in a paused room', () {
      final decision = decideSync(
        target: _state(isPlaying: false),
        local: const LocalPlayback(
          loadedTrackId: 7,
          isPlaying: true,
          position: Duration(seconds: 10),
        ),
        serverNow: _t0,
      );
      expect(decision.setPlaying, isFalse);
      expect(decision.seekTo, isNull);
    });

    test('starts and catches up a player that is behind', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(loadedTrackId: 7),
        serverNow: _t0.add(const Duration(seconds: 20)),
      );
      expect(decision.setPlaying, isTrue);
      expect(decision.seekTo, const Duration(seconds: 30));
    });

    test('leaves a player alone when it is within the threshold', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(
          loadedTrackId: 7,
          isPlaying: true,
          position: Duration(milliseconds: 30600),
        ),
        serverNow: _t0.add(const Duration(seconds: 20)),
      );
      expect(decision.isNone, isTrue);
    });

    test('corrects a drift larger than the threshold', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(
          loadedTrackId: 7,
          isPlaying: true,
          position: Duration(seconds: 27),
        ),
        serverNow: _t0.add(const Duration(seconds: 20)),
      );
      expect(decision.seekTo, const Duration(seconds: 30));
      expect(decision.setPlaying, isNull);
    });

    test('does not seek again once the track has finished locally', () {
      final decision = decideSync(
        target: _state(),
        local: const LocalPlayback(
          loadedTrackId: 7,
          isPlaying: true,
          isCompleted: true,
          position: Duration(seconds: 179),
        ),
        serverNow: _t0.add(const Duration(hours: 1)),
      );
      expect(decision.isNone, isTrue);
    });

    test('restarts a finished player when the same track starts again', () {
      final decision = decideSync(
        target: _state(positionMs: 0, timestamp: _t0),
        local: const LocalPlayback(
          loadedTrackId: 7,
          isPlaying: true,
          isCompleted: true,
          position: Duration(minutes: 3),
        ),
        serverNow: _t0,
      );
      expect(decision.seekTo, Duration.zero);
    });
  });

  group('isPastEnd', () {
    test('is true once the room clock passed the end of the track', () {
      expect(isPastEnd(_state(), _t0.add(const Duration(hours: 1))), isTrue);
    });

    test('is false in the middle of the track', () {
      expect(isPastEnd(_state(), _t0), isFalse);
    });

    test('is false when the duration is unknown', () {
      const unknown = Track(
        id: 1,
        title: 't',
        artist: 'a',
        duration: Duration.zero,
      );
      expect(
        isPastEnd(_state(track: unknown), _t0.add(const Duration(hours: 1))),
        isFalse,
      );
    });
  });
}
