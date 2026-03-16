import 'dart:async';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/controllers/room_controller.dart';

class PlayerWidget extends StatefulWidget {
  final Track? track;
  final bool initiallyPlaying;

  const PlayerWidget({
    super.key,
    required this.track,
    this.initiallyPlaying = false,
  });

  @override
  State<PlayerWidget> createState() => _PlayerWidgetState();
}

class _PlayerWidgetState extends State<PlayerWidget> {
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _startTicker();
  }

  void _startTicker() {
    _timer = Timer.periodic(const Duration(milliseconds: 100), (timer) {
      final controller = context.read<RoomController>();
      final room = controller.currentRoom;
      if (room != null && room.status == 1) {
        setState(() {});

        // Auto-skip next feature
        if (room.currentTrack != null &&
            room.currentPosition >= room.currentTrack!.duration) {
          controller.skipNext();
        }
      }
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final controller = context.watch<RoomController>();
    final room = controller.currentRoom;
    final track = room?.currentTrack;

    final isPlaying = room?.status == 1;
    final position = room?.currentPosition ?? Duration.zero;
    final duration = track?.duration ?? Duration.zero;

    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Cover art
          ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: track?.imageUrl != null
                ? Image.network(
                    track!.imageUrl,
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

          // Title & artist
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

          // Progress bar
          Column(
            children: [
              Slider(
                min: 0,
                max: duration.inMilliseconds.toDouble(),
                value: position.inMilliseconds
                    .clamp(0, duration.inMilliseconds)
                    .toDouble(),
                onChanged: (v) {
                  if (room != null) {
                    controller.seekTo(room, Duration(milliseconds: v.toInt()));
                  }
                },
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
          ),

          const SizedBox(height: 12),

          // Controls
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              IconButton(
                iconSize: 36,
                onPressed: () => controller.skipPrev(),
                icon: const Icon(Icons.skip_previous_rounded),
              ),
              const SizedBox(width: 12),
              ElevatedButton(
                onPressed: room == null || track == null
                    ? null
                    : () => controller.togglePlay(room),
                style: ElevatedButton.styleFrom(
                  shape: const CircleBorder(),
                  padding: const EdgeInsets.all(14),
                ),
                child: Icon(
                  isPlaying ? Icons.pause : Icons.play_arrow,
                  size: 28,
                ),
              ),
              const SizedBox(width: 12),
              IconButton(
                iconSize: 36,
                onPressed: () => controller.skipNext(),
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
