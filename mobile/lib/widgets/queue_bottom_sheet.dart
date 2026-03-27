import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../core/theme.dart';
import '../widgets/track_list_tile.dart';

void showQueueBottomSheet(BuildContext context) {
  final room = context.read<RoomController>().currentRoom;
  if (room == null) return;

  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    builder: (ctx) {
      return Consumer<RoomController>(
        builder: (ctx2, controller, _) {
          final currentRoom = controller.currentRoom;
          if (currentRoom == null) return const SizedBox.shrink();
          final queue = currentRoom.queue.toList();
          queue.sort((a, b) => a.position.compareTo(b.position));

          return SizedBox(
            height: MediaQuery.of(ctx).size.height * 0.6,
            child: Column(
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(
                    vertical: AppTheme.spacingMd,
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      const Text(
                        'Incoming Tracks',
                        style: TextStyle(fontWeight: FontWeight.bold),
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: ReorderableListView(
                    onReorder: (oldIndex, newIndex) {
                      if (newIndex > oldIndex) newIndex -= 1;
                      
                      final item = queue[oldIndex];
                      double newPos;
                      if (newIndex == 0) {
                         newPos = queue.isNotEmpty ? queue.first.position - 100 : 0;
                      } else if (newIndex == queue.length - 1) {
                         newPos = queue.last.position + 100;
                      } else {
                         final prev = queue[newIndex > oldIndex ? newIndex : newIndex - 1].position;
                         final next = queue[newIndex > oldIndex ? newIndex + 1 : newIndex].position;
                         newPos = (prev + next) / 2;
                      }
                      
                      controller.moveQueueItem(currentRoom, item, newPos);
                    },
                    children: [
                      for (var i = 0; i < queue.length; i++)
                        Container(
                          key: ValueKey(queue[i].id),
                          child: TrackListTile(
                            track: queue[i].track,
                            trailingIcon: Icons.delete_outline,
                            onTapTrailing: () =>
                                controller.removeQueueItem(currentRoom, queue[i]),
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          );
        },
      );
    },
  );
}
