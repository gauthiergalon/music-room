import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';

import '../core/logger/logger.dart';
import '../models/track.dart';

/// What the room logic needs from an audio player.
abstract class PlaybackEngine {
  bool get isPlaying;

  /// The loaded track has been played to its end.
  bool get isCompleted;

  Duration get position;
  Stream<Duration> get positionStream;
  Stream<Duration?> get durationStream;

  /// Emits once each time playback reaches the end of the loaded track.
  Stream<void> get completedStream;

  /// Loads [track] from [streamUrl] at [initialPosition] without starting
  /// playback. Completes when the source is ready.
  Future<void> load(
    Track track,
    String streamUrl, {
    Duration initialPosition = Duration.zero,
  });

  void play();
  void pause();
  Future<void> seek(Duration position);

  /// Stops playback and unloads the current track.
  Future<void> stop();
}

/// Audio player. Notifies its listeners when the current track changes.
class AudioService extends ChangeNotifier implements PlaybackEngine {
  final AudioPlayer _audioPlayer = AudioPlayer();
  StreamSubscription<PlaybackEvent>? _eventSubscription;
  Track? _currentTrack;

  AudioService() {
    _eventSubscription = _audioPlayer.playbackEventStream.listen(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        logger.error('Audio playback error', error: error);
      },
    );
  }

  Track? get currentTrack => _currentTrack;

  @override
  bool get isPlaying => _audioPlayer.playing;

  @override
  bool get isCompleted =>
      _audioPlayer.processingState == ProcessingState.completed;

  @override
  Duration get position => _audioPlayer.position;

  @override
  Stream<Duration> get positionStream => _audioPlayer.positionStream;

  @override
  Stream<Duration?> get durationStream => _audioPlayer.durationStream;

  @override
  Stream<void> get completedStream => _audioPlayer.processingStateStream
      .map((state) => state == ProcessingState.completed)
      .distinct()
      .where((completed) => completed);

  @override
  Future<void> load(
    Track track,
    String streamUrl, {
    Duration initialPosition = Duration.zero,
  }) async {
    logger.debug('Audio loading: ${track.title} at $initialPosition');

    // A playing player would start the new source as soon as it is loaded;
    // whether to play is decided by the caller once the load is done.
    if (_audioPlayer.playing) await _audioPlayer.pause();

    _setCurrentTrack(track);

    final imageUrl = track.imageUrl;
    await _audioPlayer.setAudioSource(
      AudioSource.uri(
        Uri.parse(streamUrl),
        tag: MediaItem(
          id: track.id.toString(),
          title: track.title,
          artist: track.artist,
          artUri: imageUrl != null ? Uri.parse(imageUrl) : null,
        ),
      ),
      initialPosition: initialPosition,
    );
  }

  @override
  void play() {
    logger.debug('Audio play');
    // The future of `play()` only completes when playback stops.
    unawaited(
      _audioPlayer.play().catchError((Object error) {
        logger.error('Audio play failed', error: error);
      }),
    );
  }

  @override
  void pause() {
    logger.debug('Audio pause');
    unawaited(_audioPlayer.pause());
  }

  @override
  Future<void> seek(Duration position) => _audioPlayer.seek(position);

  @override
  Future<void> stop() async {
    logger.debug('Audio stop');
    _setCurrentTrack(null);
    await _audioPlayer.stop();
  }

  void _setCurrentTrack(Track? track) {
    if (_currentTrack == track) return;
    _currentTrack = track;
    notifyListeners();
  }

  @override
  void dispose() {
    _eventSubscription?.cancel();
    _audioPlayer.dispose();
    super.dispose();
  }
}
