class Track {
  final int id;
  final String title;
  final String artist;
  final String? imageUrl;
  final Duration duration;

  Track({
    required this.id,
    required this.title,
    required this.artist,
    this.imageUrl,
    required this.duration,
  });
}
