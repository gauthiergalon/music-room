import 'package:flutter/material.dart';
import '../models/room.dart';
import '../core/theme.dart';

class RoomListItem extends StatelessWidget {
  final Room room;
  final VoidCallback onTap;

  const RoomListItem({super.key, required this.room, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  room.isPlaying
                      ? Icons.play_arrow_rounded
                      : Icons.pause_rounded,
                  color: theme.colorScheme.onPrimary,
                  size: 20,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Text(
                  room.name,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              if (!room.isPublic) ...[
                Icon(
                  Icons.lock_outline_rounded,
                  color: theme.colorScheme.onSurfaceVariant,
                  size: 18,
                  semanticLabel: 'Private room',
                ),
                const SizedBox(width: 8),
              ],
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
