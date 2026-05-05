import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../core/theme.dart';
import '../core/utils/ui_utils.dart';
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
                    onReorder: (oldIndex, newIndex) async {
                      try {
                        await controller.reorderQueueItem(
                          currentRoom,
                          queue,
                          oldIndex,
                          newIndex,
                        );
                      } catch (_) {
                        if (ctx2.mounted) {
                          UiUtils.showError(ctx2, 'Failed to reorder queue.');
                        }
                      }
                    },
                    children: [
                      for (final item in queue)
                        Dismissible(
                          key: ValueKey(item.id),
                          direction: DismissDirection.endToStart,
                          background: Container(
                            alignment: Alignment.centerRight,
                            padding: const EdgeInsets.symmetric(horizontal: 20),
                            color: Colors.red,
                            child: const Icon(
                              Icons.delete_outline,
                              color: Colors.white,
                            ),
                          ),
                          onDismissed: (_) async {
                            try {
                              await controller.removeQueueItem(
                                currentRoom,
                                item,
                              );
                            } catch (_) {
                              if (ctx2.mounted) {
                                UiUtils.showError(
                                  ctx2,
                                  'Failed to remove song from queue.',
                                );
                              }
                            }
                          },
                          child: TrackListTile(track: item.track),
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
