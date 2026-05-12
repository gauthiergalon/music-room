import 'package:flutter/widgets.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';
import '../core/logger/logger.dart';
import '../models/track.dart';

class AudioService extends ChangeNotifier {
  final AudioPlayer _audioPlayer = AudioPlayer();
  int _playbackRequestId = 0;
  Track? _currentTrack;

  bool get isPlaying => _audioPlayer.playing;
  Duration get position => _audioPlayer.position;
  Duration? get duration => _audioPlayer.duration;
  Track? get currentTrack => _currentTrack;
  AudioPlayer get player => _audioPlayer;

  Stream<PlayerState> get playerStateStream => _audioPlayer.playerStateStream;

  AudioService() {
    _audioPlayer.playerStateStream.listen((state) {
      notifyListeners();
    });
  }

  Future<void> playTrack(Track track, Duration position) async {
    final requestId = ++_playbackRequestId;

    final streamUrl = track.streamUrl;

    if (requestId != _playbackRequestId) return;

    await _audioPlayer.stop();

    if (requestId != _playbackRequestId) return;

    logger.debug('Audio playing: ${track.title}');

    final audioSource = AudioSource.uri(
      Uri.parse(streamUrl),
      tag: MediaItem(
        id: track.id.toString(),
        title: track.title,
        artist: track.artist,
        artUri: track.imageUrl != null ? Uri.parse(track.imageUrl!) : null,
      ),
    );

    await _audioPlayer.setLoopMode(LoopMode.off);
    await _audioPlayer.setAudioSource(audioSource);

    if (requestId != _playbackRequestId) return;

    _currentTrack = track;
    await _audioPlayer.seek(position);
    await _audioPlayer.play();

    notifyListeners();
  }

  void play() {
    logger.debug('Audio play');
    _audioPlayer.play();
  }

  void pause() {
    logger.debug('Audio pause');
    _audioPlayer.pause();
  }

  void stop() {
    logger.debug('Audio stop');
    _audioPlayer.stop();
  }

  void seek(Duration position) => _audioPlayer.seek(position);

  @override
  void dispose() {
    _audioPlayer.dispose();
    super.dispose();
  }
}
