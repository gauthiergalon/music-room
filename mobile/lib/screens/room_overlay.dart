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
          Center(
            child: PlayerWidget(
              track: room.currentTrack,
              initiallyPlaying: room.status == 1,
            ),
          ),
          Positioned(
            top: AppTheme.spacingSm,
            left: 0,
            right: 0,
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: AppTheme.spacingSm,
              ),
              child: Row(
                children: [
                  Material(
                    color: Colors.transparent,
                    child: IconButton(
                      tooltip: 'Leave room',
                      icon: const Icon(Icons.exit_to_app),
                      onPressed: () {
                        controller.leaveRoom();
                      },
                      color: Colors.red,
                    ),
                  ),
                  const Spacer(),
                  Expanded(
                    flex: 2,
                    child: Center(
                      child: Text(
                        room.name,
                        style: Theme.of(context).textTheme.titleMedium,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                  const Spacer(),
                  Material(
                    color: Colors.transparent,
                    child: IconButton(
                      tooltip: 'Listeners',
                      icon: const Icon(Icons.people),
                      onPressed: () => showListenersDialog(context),
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
