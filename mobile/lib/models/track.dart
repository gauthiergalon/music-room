import 'track_cache.dart';

class Track {
  final int id;
  final String title;
  final String artist;
  final String? imageUrl;
  final Duration duration;
  final String streamUrl; // L'URL de la musique pour just_audio

  Track({
    required this.id,
    required this.title,
    required this.artist,
    this.imageUrl,
    required this.duration,
    this.streamUrl =
        'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3', // Vraie musique de test
  });

  factory Track.fromJson(Map<String, dynamic> json) {
    String artistName = 'Unknown Artist';
    if (json['artists'] != null && (json['artists'] as List).isNotEmpty) {
      artistName = json['artists'][0]['name'] ?? 'Unknown Artist';
    }

    String? coverUrl;
    if (json['album'] != null && json['album']['cover'] != null) {
      final String cover = json['album']['cover'];
      final formattedCover = cover.replaceAll('-', '/');
      coverUrl =
          'https://resources.tidal.com/images/$formattedCover/320x320.jpg';
    }

    final track = Track(
      id: json['id'],
      title: json['title'] ?? 'Unknown Title',
      artist: artistName,
      duration: Duration(seconds: json['duration'] ?? 0),
      imageUrl: coverUrl,
    );

    // Save to local cache so the queue can display it properly
    cacheTrack(track);
    return track;
  }
}
