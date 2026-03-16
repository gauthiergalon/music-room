import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';

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

          return SizedBox(
            height: MediaQuery.of(ctx).size.height * 0.6,
            child: Column(
              children: [
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16),
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
                      controller.moveTrack(currentRoom, oldIndex, newIndex);
                    },
                    children: [
                      for (var i = 0; i < queue.length; i++)
                        ListTile(
                          key: ValueKey(queue[i].title),
                          leading: ClipRRect(
                            borderRadius: BorderRadius.circular(6),
                            child: Image.network(
                              queue[i].imageUrl,
                              width: 56,
                              height: 56,
                              fit: BoxFit.cover,
                            ),
                          ),
                          title: Text(queue[i].title),
                          subtitle: Text(queue[i].artist),
                          trailing: IconButton(
                            icon: const Icon(Icons.delete_outline),
                            onPressed: () =>
                                controller.removeTrack(currentRoom, i),
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
