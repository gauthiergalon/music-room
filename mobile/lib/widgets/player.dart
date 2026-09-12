import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:mobile/controllers/room_controller.dart';
import '../controllers/auth_controller.dart';
import '../core/theme.dart';

class PlayerWidget extends StatefulWidget {
  const PlayerWidget({super.key});

  @override
  State<PlayerWidget> createState() => _PlayerWidgetState();
}

class _PlayerWidgetState extends State<PlayerWidget> {
  double? _dragValue;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final controller = context.watch<RoomController>();
    final room = controller.currentRoom;
    final track = room?.currentTrack;

    final isPlaying = controller.isPlaying;
    final auth = context.read<AuthController>();
    final myId = auth.user?.id;
    final delegateId = controller.currentRoom?.delegateUserId;
    final ownerId = controller.currentRoom?.owner;
    final canControl = (delegateId != null && delegateId == myId) ||
        (delegateId == null && ownerId == myId);

    return Padding(
      padding: AppTheme.paddingLg,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: track?.imageUrl != null
                ? Image.network(
                    track!.imageUrl!,
                    width: 260,
                    height: 260,
                    fit: BoxFit.cover,
                    errorBuilder: (context, error, stack) => Container(
                      width: 260,
                      height: 260,
                      color: theme.colorScheme.surfaceContainerHighest,
                      child: const Icon(Icons.music_note, size: 72),
                    ),
                  )
                : Container(
                    width: 260,
                    height: 260,
                    color: theme.colorScheme.surfaceContainerHighest,
                    child: Icon(
                      Icons.music_note,
                      size: 72,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
          ),
          const SizedBox(height: 20),

          Column(
            children: [
              Text(
                track?.title ?? 'No track',
                style: theme.textTheme.titleLarge,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 6),
              Text(
                track?.artist ?? '',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),

          const SizedBox(height: 18),

          StreamBuilder<Duration>(
            stream: controller.positionStream,
            builder: (context, snapshot) {
              final position = snapshot.data ?? controller.playbackPosition;
              final duration =
                  controller.playbackDuration ??
                  track?.duration ??
                  Duration.zero;
              final sliderMax = duration.inMilliseconds > 0
                  ? duration.inMilliseconds.toDouble()
                  : 1.0;
              final sliderValue = position.inMilliseconds
                  .clamp(
                    0,
                    duration.inMilliseconds > 0 ? duration.inMilliseconds : 1,
                  )
                  .toDouble();

              return Column(
                children: [
                  SliderTheme(
                    data: SliderThemeData(
                      activeTrackColor: theme.colorScheme.primary,
                      inactiveTrackColor: theme.colorScheme.surfaceContainerHighest,
                      thumbColor: theme.colorScheme.primary,
                      overlayColor: theme.colorScheme.primary.withValues(alpha: 0.2),
                      valueIndicatorColor: theme.colorScheme.secondaryContainer,
                    ),
                    child: Slider(
                      min: 0,
                      max: sliderMax,
                      value: _dragValue ?? sliderValue,
                      onChangeStart: (v) {
                        setState(() {
                          _dragValue = v;
                        });
                      },
                      onChanged: canControl ? (v) {
                        setState(() {
                          _dragValue = v;
                        });
                      } : null,
                      onChangeEnd: canControl ? (v) {
                        setState(() {
                          _dragValue = null;
                        });
                        if (room != null) {
                          controller.seekTo(
                            room,
                            Duration(milliseconds: v.toInt()),
                          );
                        }
                      } : null,
                    ),
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        _formatDuration(position),
                        style: theme.textTheme.bodySmall,
                      ),
                      Text(
                        _formatDuration(duration),
                        style: theme.textTheme.bodySmall,
                      ),
                    ],
                  ),
                ],
              );
            },
          ),

          const SizedBox(height: 12),

          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              IconButton(
                iconSize: 36,
                onPressed: canControl ? () => controller.skipPrev() : null,
                icon: const Icon(Icons.skip_previous_rounded),
              ),
              const SizedBox(width: 12),
              FilledButton(
                onPressed: canControl && room != null && track != null
                    ? () => controller.togglePlay(room)
                    : null,
                style: FilledButton.styleFrom(
                  shape: const CircleBorder(),
                  padding: AppTheme.paddingMd,
                  backgroundColor: theme.colorScheme.secondaryContainer,
                  foregroundColor: theme.colorScheme.onSecondaryContainer,
                  elevation: 2,
                ),
                child: Icon(
                  isPlaying ? Icons.pause_rounded : Icons.play_arrow_rounded,
                  size: 28,
                ),
              ),
              const SizedBox(width: 12),
              IconButton(
                iconSize: 36,
                onPressed: canControl ? () => controller.skipNext() : null,
                icon: const Icon(Icons.skip_next_rounded),
              ),
            ],
          ),
        ],
      ),
    );
  }

  String _formatDuration(Duration d) {
    // Only display seconds since milliseconds are for slider granularity
    final m = d.inMinutes.remainder(60).toString().padLeft(2, '0');
    final s = d.inSeconds.remainder(60).toString().padLeft(2, '0');
    return '$m:$s';
  }
}
