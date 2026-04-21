import 'track.dart';

final Map<int, Track> mockTracks = {
  111: Track(
    id: 111,
    title: 'Song 111',
    artist: 'Artist',
    duration: const Duration(minutes: 3),
  ),
  222: Track(
    id: 222,
    title: 'Song 222',
    artist: 'Artist',
    duration: const Duration(minutes: 3),
  ),
  333: Track(
    id: 333,
    title: 'Song 333',
    artist: 'Artist',
    duration: const Duration(minutes: 3),
  ),
  12345: Track(
    id: 12345,
    title: 'Song 12345',
    artist: 'Artist',
    duration: const Duration(minutes: 3),
  ),
};

Track getMockTrack(int id) {
  return mockTracks[id] ??
      Track(
        id: id,
        title: 'Unknown Song $id',
        artist: 'Unknown',
        duration: const Duration(minutes: 3),
      );
}
