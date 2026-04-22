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
}
