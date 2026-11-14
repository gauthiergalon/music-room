class Track {
  final int id;
  final String title;
  final String artist;
  final String? imageUrl;
  final Duration duration;

  const Track({
    required this.id,
    required this.title,
    required this.artist,
    this.imageUrl,
    required this.duration,
  });

  factory Track.fromJson(Map<String, dynamic> json) {
    var artistName = 'Unknown Artist';
    final artists = json['artists'];
    if (artists is List && artists.isNotEmpty && artists.first is Map) {
      artistName = (artists.first as Map)['name']?.toString() ?? artistName;
    }

    String? coverUrl;
    final album = json['album'];
    if (album is Map && album['cover'] is String) {
      final formattedCover = (album['cover'] as String).replaceAll('-', '/');
      coverUrl =
          'https://resources.tidal.com/images/$formattedCover/320x320.jpg';
    }

    return Track(
      id: (json['id'] as num).toInt(),
      title: json['title']?.toString() ?? 'Unknown Title',
      artist: artistName,
      duration: Duration(seconds: (json['duration'] as num?)?.toInt() ?? 0),
      imageUrl: coverUrl,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Track &&
          id == other.id &&
          title == other.title &&
          artist == other.artist &&
          imageUrl == other.imageUrl &&
          duration == other.duration;

  @override
  int get hashCode => Object.hash(id, title, artist, imageUrl, duration);
}
