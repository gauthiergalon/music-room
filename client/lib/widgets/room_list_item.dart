import 'package:flutter/material.dart';
import '../models/room.dart';
import '../core/theme.dart';

class RoomListItem extends StatelessWidget {
  final Room room;
  final VoidCallback onTap;

  const RoomListItem({super.key, required this.room, required this.onTap});

  static const double _coverSize = 48;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final track = room.currentTrack;

    return Card.filled(
      elevation: 0,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(16),
        child: Padding(
          padding: AppTheme.paddingMd,
          child: Row(
            children: [
              _Cover(imageUrl: track?.imageUrl, size: _coverSize),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      room.name,
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 2),
                    Row(
                      children: [
                        if (track != null) ...[
                          Icon(
                            room.isPlaying
                                ? Icons.play_arrow_rounded
                                : Icons.pause_rounded,
                            size: 16,
                            color: theme.colorScheme.primary,
                            semanticLabel: room.isPlaying
                                ? 'Playing'
                                : 'Paused',
                          ),
                          const SizedBox(width: 4),
                        ],
                        Expanded(
                          child: Text(
                            track == null
                                ? 'Nothing playing'
                                : '${track.title} · ${track.artist}',
                            style: theme.textTheme.bodyMedium?.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              if (!room.isPublic) ...[
                const SizedBox(width: 8),
                Icon(
                  Icons.lock_outline_rounded,
                  color: theme.colorScheme.onSurfaceVariant,
                  size: 18,
                  semanticLabel: 'Private room',
                ),
              ],
              const SizedBox(width: 8),
              Icon(
                Icons.chevron_right_rounded,
                color: theme.colorScheme.onSurfaceVariant,
                size: 20,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Cover of the room's current track, or a placeholder when there is none.
class _Cover extends StatelessWidget {
  const _Cover({required this.imageUrl, required this.size});

  final String? imageUrl;
  final double size;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final placeholder = Container(
      width: size,
      height: size,
      color: theme.colorScheme.surfaceContainerHighest,
      child: Icon(
        Icons.music_note_rounded,
        color: theme.colorScheme.onSurfaceVariant,
        size: 22,
      ),
    );

    return ClipRRect(
      borderRadius: BorderRadius.circular(12),
      child: imageUrl == null
          ? placeholder
          : Image.network(
              imageUrl!,
              width: size,
              height: size,
              cacheWidth: (size * 3).round(),
              fit: BoxFit.cover,
              errorBuilder: (context, error, stack) => placeholder,
            ),
    );
  }
}
