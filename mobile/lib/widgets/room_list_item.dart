import 'package:flutter/material.dart';
import '../models/room.dart';
import '../core/theme.dart';

class RoomListItem extends StatelessWidget {
  final Room room;
  final VoidCallback onTap;

  const RoomListItem({super.key, required this.room, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: AppTheme.spacingMd,
        vertical: 6.0,
      ),
      child: FilledButton(
        onPressed: onTap,
        style: FilledButton.styleFrom(
          minimumSize: const Size(double.infinity, 56),
          alignment: Alignment.centerLeft,
          padding: const EdgeInsets.symmetric(horizontal: AppTheme.spacingMd),
        ),
        child: Row(
          children: [
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: Colors.white.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Icon(
                room.status == 1 ? Icons.play_arrow : Icons.pause,
                color: Colors.white70,
                size: 18,
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                room.name,
                style: const TextStyle(fontWeight: FontWeight.bold),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
