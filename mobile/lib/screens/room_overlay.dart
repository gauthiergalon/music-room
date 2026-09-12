import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../widgets/player.dart';
import '../widgets/listeners_dialog.dart';
import '../widgets/queue_bottom_sheet.dart';
import '../core/theme.dart';

class RoomOverlay extends StatelessWidget {
  const RoomOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<RoomController>();
    final room = controller.currentRoom;
    if (room == null) return const SizedBox.shrink();

    return Scaffold(
      backgroundColor: Theme.of(context).scaffoldBackgroundColor,
      body: Stack(
        children: [
          Center(child: const PlayerWidget()),
          Positioned(
            top: AppTheme.spacingSm,
            left: 0,
            right: 0,
            child: Padding(
              padding: AppTheme.paddingMd,
              child: Row(
                children: [
                  IconButton(
                    tooltip: 'Leave room',
                    icon: const Icon(Icons.exit_to_app_rounded),
                    onPressed: () {
                      controller.leaveRoom();
                    },
                    style: IconButton.styleFrom(
                      backgroundColor: Theme.of(context).colorScheme.errorContainer,
                      foregroundColor: Theme.of(context).colorScheme.onErrorContainer,
                    ),
                  ),
                  const Spacer(),
                  Expanded(
                    flex: 2,
                    child: Center(
                      child: Text(
                        room.name,
                        style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    tooltip: 'Listeners',
                    icon: const Icon(Icons.people_rounded),
                    onPressed: () => showListenersDialog(context),
                    style: IconButton.styleFrom(
                      backgroundColor: Theme.of(context).colorScheme.secondaryContainer,
                      foregroundColor: Theme.of(context).colorScheme.onSecondaryContainer,
                    ),
                  ),
                ],
              ),
            ),
          ),
          Positioned(
            right: AppTheme.spacingMd,
            bottom: AppTheme.spacingMd,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                FloatingActionButton(
                  heroTag: 'queue',
                  onPressed: () => showQueueBottomSheet(context),
                  child: const Icon(Icons.queue_music),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
