import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../core/theme.dart';
import '../core/utils/ui_utils.dart';
import '../models/queue_item.dart';
import '../widgets/track_list_tile.dart';

void showQueueBottomSheet(BuildContext context) {
  final room = context.read<RoomController>().currentRoom;
  if (room == null) return;

  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    shape: const RoundedRectangleBorder(
      borderRadius: BorderRadius.vertical(top: Radius.circular(28)),
    ),
    builder: (ctx) {
      // Rebuilds only when the queue itself is replaced.
      return Selector<RoomController, List<QueueItem>?>(
        selector: (_, c) => c.currentRoom?.queue,
        builder: (ctx2, queue, _) {
          if (queue == null) return const SizedBox.shrink();
          final controller = ctx2.read<RoomController>();

          return SizedBox(
            height: MediaQuery.of(ctx).size.height * 0.6,
            child: Column(
              children: [
                Padding(
                  padding: AppTheme.paddingMd,
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
                    onReorderItem: (oldIndex, newIndex) async {
                      try {
                        await controller.reorderQueueItem(oldIndex, newIndex);
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
                            padding: AppTheme.paddingMd,
                            color: Colors.red,
                            child: const Icon(
                              Icons.delete_outline,
                              color: Colors.white,
                            ),
                          ),
                          onDismissed: (_) async {
                            try {
                              await controller.removeQueueItem(item);
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
