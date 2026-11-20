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
      // Rebuilds only when the queue itself is replaced or the right to
      // edit it changes.
      return Selector<RoomController, (List<QueueItem>?, bool)>(
        selector: (_, c) => (c.currentRoom?.queue, c.canEditQueue),
        builder: (ctx2, selected, _) {
          final (queue, canEdit) = selected;
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
                if (!canEdit)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: Text(
                      lockedQueueMessage,
                      textAlign: TextAlign.center,
                      style: Theme.of(ctx2).textTheme.bodySmall,
                    ),
                  ),
                Expanded(
                  child: !canEdit
                      ? ListView(
                          children: [
                            for (final item in queue)
                              TrackListTile(
                                key: ValueKey(item.id),
                                track: item.track,
                              ),
                          ],
                        )
                      : ReorderableListView(
                          onReorderItem: (oldIndex, newIndex) async {
                            try {
                              await controller.reorderQueueItem(
                                oldIndex,
                                newIndex,
                              );
                            } catch (_) {
                              if (ctx2.mounted) {
                                UiUtils.showError(
                                  ctx2,
                                  'Failed to reorder queue.',
                                );
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
