import 'track.dart';

final Map<int, Track> trackCache = {};

Track getCachedTrack(int id) {
  return trackCache[id] ??
      Track(
        id: id,
        title: 'Song $id (cached missing)',
        artist: 'Unknown Artist',
        duration: const Duration(minutes: 3),
      );
}

void cacheTrack(Track track) {
  trackCache[track.id] = track;
}
