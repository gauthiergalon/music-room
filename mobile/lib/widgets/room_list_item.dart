import 'package:flutter/material.dart';
import '../models/room.dart';

class RoomListItem extends StatelessWidget {
  final Room room;
  final VoidCallback onTap;

  const RoomListItem({super.key, required this.room, required this.onTap});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: FilledButton.tonal(
        onPressed: onTap,
        style: FilledButton.styleFrom(
          minimumSize: const Size(double.infinity, 72),
          alignment: Alignment.centerLeft,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        ),
        child: Row(
          children: [
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    room.owner,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      const Icon(Icons.music_note, size: 16),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          room.currentTrack != null
                              ? "${room.currentTrack!.title} - ${room.currentTrack!.artist}"
                              : 'No track playing',
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const Icon(Icons.people),
            const SizedBox(width: 4),
            Text(room.people.toString()),
          ],
        ),
      ),
    );
  }
}
