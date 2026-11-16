import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../controllers/room_controller.dart';
import '../core/theme.dart';
import '../models/track.dart';

/// The room's player, bound to the [RoomController].
class PlayerWidget extends StatelessWidget {
  const PlayerWidget({super.key});

  @override
  Widget build(BuildContext context) {
    // Rebuild only when what is shown here changes; the position has its
    // own stream inside the view.
    final (track, isPlaying, canControl) = context
        .select<RoomController, (Track?, bool, bool)>(
          (c) => (c.currentTrack, c.isPlaying, c.canControl),
        );
    final controller = context.read<RoomController>();

    return PlayerView(
      track: track,
      isPlaying: isPlaying,
      canControl: canControl,
      positionStream: controller.positionStream,
      currentPosition: () => controller.playbackPosition,
      onSeek: controller.seekTo,
      onTogglePlay: controller.togglePlay,
      onSkipNext: controller.skipNext,
      onSkipPrevious: controller.skipPrev,
    );
  }
}

/// Cover, track details and transport controls.
///
/// The cover takes the space the controls leave: it sits above them on a tall
/// area and beside them on a wide one, shrinks as the area gets smaller and
/// disappears before anything overflows.
class PlayerView extends StatelessWidget {
  const PlayerView({
    super.key,
    required this.track,
    required this.isPlaying,
    required this.canControl,
    required this.positionStream,
    required this.currentPosition,
    required this.onSeek,
    required this.onTogglePlay,
    required this.onSkipNext,
    required this.onSkipPrevious,
  });

  final Track? track;
  final bool isPlaying;
  final bool canControl;
  final Stream<Duration> positionStream;
  final Duration Function() currentPosition;
  final ValueChanged<Duration> onSeek;
  final VoidCallback onTogglePlay;
  final VoidCallback onSkipNext;
  final VoidCallback onSkipPrevious;

  /// The cover never grows past this, however large the screen is.
  static const double maxCoverSize = 420;

  /// Below this the cover is not worth showing.
  static const double minCoverSize = 96;

  /// Widest the details and controls get, so the slider stays usable on a
  /// desktop-sized window.
  static const double maxControlsWidth = 480;

  /// Narrowest area in which cover and controls fit side by side.
  static const double minSideBySideWidth = 560;

  static const double _coverGap = 20;
  static const double _sideGap = 32;

  @override
  Widget build(BuildContext context) {
    final controls = _PlayerControls(
      track: track,
      isPlaying: isPlaying,
      canControl: canControl,
      positionStream: positionStream,
      currentPosition: currentPosition,
      onSeek: onSeek,
      onTogglePlay: onTogglePlay,
      onSkipNext: onSkipNext,
      onSkipPrevious: onSkipPrevious,
    );

    return Padding(
      padding: AppTheme.paddingLg,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final height = constraints.maxHeight;
          final sideBySide = width > height && width >= minSideBySideWidth;

          return sideBySide
              ? _buildSideBySide(controls, width, height)
              : _buildStacked(controls, height);
        },
      ),
    );
  }

  /// Wide area (landscape, desktop): the cover is bounded by the height.
  Widget _buildSideBySide(Widget controls, double width, double height) {
    final coverSize = math.min(
      maxCoverSize,
      math.min(height, (width - _sideGap) * 0.45),
    );

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        if (coverSize >= minCoverSize) ...[
          _Cover(track: track, size: coverSize),
          const SizedBox(width: _sideGap),
        ],
        Flexible(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: maxControlsWidth),
            // Scrolls instead of overflowing when even the controls alone
            // are taller than the area (very short windows).
            child: Center(child: SingleChildScrollView(child: controls)),
          ),
        ),
      ],
    );
  }

  /// Tall area (portrait): the cover gets the height the controls leave.
  Widget _buildStacked(Widget controls, double height) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: maxControlsWidth),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Flexible(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final coverSize = math.min(
                    maxCoverSize,
                    math.min(
                      constraints.maxWidth,
                      constraints.maxHeight - _coverGap,
                    ),
                  );
                  if (coverSize < minCoverSize) return const SizedBox.shrink();

                  return Padding(
                    padding: const EdgeInsets.only(bottom: _coverGap),
                    child: _Cover(track: track, size: coverSize),
                  );
                },
              ),
            ),
            // The controls keep their natural height and the cover adapts.
            // They scroll if the area cannot even hold them.
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: height),
              child: SingleChildScrollView(child: controls),
            ),
          ],
        ),
      ),
    );
  }
}

class _Cover extends StatelessWidget {
  const _Cover({required this.track, required this.size});

  final Track? track;
  final double size;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final imageUrl = track?.imageUrl;

    final placeholder = ColoredBox(
      color: theme.colorScheme.surfaceContainerHighest,
      child: Center(
        child: Icon(
          Icons.music_note,
          size: size * 0.3,
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );

    return SizedBox.square(
      key: const ValueKey('player-cover'),
      dimension: size,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(12),
        child: imageUrl != null
            ? Image.network(
                imageUrl,
                fit: BoxFit.cover,
                errorBuilder: (context, error, stack) => placeholder,
              )
            : placeholder,
      ),
    );
  }
}

class _PlayerControls extends StatefulWidget {
  const _PlayerControls({
    required this.track,
    required this.isPlaying,
    required this.canControl,
    required this.positionStream,
    required this.currentPosition,
    required this.onSeek,
    required this.onTogglePlay,
    required this.onSkipNext,
    required this.onSkipPrevious,
  });

  final Track? track;
  final bool isPlaying;
  final bool canControl;
  final Stream<Duration> positionStream;
  final Duration Function() currentPosition;
  final ValueChanged<Duration> onSeek;
  final VoidCallback onTogglePlay;
  final VoidCallback onSkipNext;
  final VoidCallback onSkipPrevious;

  @override
  State<_PlayerControls> createState() => _PlayerControlsState();
}

class _PlayerControlsState extends State<_PlayerControls> {
  double? _dragValue;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final track = widget.track;
    final canControlTrack = widget.canControl && track != null;

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          track?.title ?? 'No track',
          style: theme.textTheme.titleLarge,
          textAlign: TextAlign.center,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 6),
        Text(
          track?.artist ?? '',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 18),
        StreamBuilder<Duration>(
          stream: widget.positionStream,
          builder: (context, snapshot) {
            final duration = track?.duration ?? Duration.zero;
            final position = track == null
                ? Duration.zero
                : snapshot.data ?? widget.currentPosition();
            final maxMs = math.max(duration.inMilliseconds, 1);
            final sliderValue = position.inMilliseconds
                .clamp(0, maxMs)
                .toDouble();

            return Column(
              children: [
                SliderTheme(
                  data: SliderThemeData(
                    activeTrackColor: theme.colorScheme.primary,
                    inactiveTrackColor:
                        theme.colorScheme.surfaceContainerHighest,
                    thumbColor: theme.colorScheme.primary,
                    overlayColor: theme.colorScheme.primary.withValues(
                      alpha: 0.2,
                    ),
                    valueIndicatorColor: theme.colorScheme.secondaryContainer,
                  ),
                  child: Slider(
                    min: 0,
                    max: maxMs.toDouble(),
                    value: _dragValue ?? sliderValue,
                    onChanged: canControlTrack
                        ? (value) => setState(() => _dragValue = value)
                        : null,
                    onChangeEnd: canControlTrack
                        ? (value) {
                            setState(() => _dragValue = null);
                            widget.onSeek(
                              Duration(milliseconds: value.toInt()),
                            );
                          }
                        : null,
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
              onPressed: canControlTrack ? widget.onSkipPrevious : null,
              icon: const Icon(Icons.skip_previous_rounded),
            ),
            const SizedBox(width: 12),
            FilledButton(
              onPressed: canControlTrack ? widget.onTogglePlay : null,
              style: FilledButton.styleFrom(
                shape: const CircleBorder(),
                padding: AppTheme.paddingMd,
                backgroundColor: theme.colorScheme.primary,
                foregroundColor: theme.colorScheme.onPrimary,
                elevation: 2,
              ),
              child: Icon(
                widget.isPlaying
                    ? Icons.pause_rounded
                    : Icons.play_arrow_rounded,
                size: 28,
              ),
            ),
            const SizedBox(width: 12),
            IconButton(
              iconSize: 36,
              onPressed: widget.canControl ? widget.onSkipNext : null,
              icon: const Icon(Icons.skip_next_rounded),
            ),
          ],
        ),
      ],
    );
  }

  String _formatDuration(Duration d) {
    // Only display seconds since milliseconds are for slider granularity
    final m = d.inMinutes.remainder(60).toString().padLeft(2, '0');
    final s = d.inSeconds.remainder(60).toString().padLeft(2, '0');
    return '$m:$s';
  }
}
