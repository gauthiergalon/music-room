import 'package:just_audio/just_audio.dart';
import 'package:just_audio_background/just_audio_background.dart';

void main() async {
  final silenceSource = SilenceAudioSource(
        duration: const Duration(hours: 24),
        tag: MediaItem(
          id: 'silence_placeholder',
          title: 'Music Room Active',
        ),
  );
  print('tag type: ${silenceSource.tag.runtimeType}');
  print('is MediaItem: ${silenceSource.tag is MediaItem}');
}
