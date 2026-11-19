import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../services/websocket_service.dart';
import '../widgets/player.dart';
import '../widgets/listeners_dialog.dart';
import '../widgets/queue_bottom_sheet.dart';
import '../core/theme.dart';

class RoomOverlay extends StatelessWidget {
  const RoomOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    final roomName = context.select<RoomController, String?>(
      (c) => c.currentRoom?.name,
    );
    if (roomName == null) return const SizedBox.shrink();

    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: theme.scaffoldBackgroundColor,
      floatingActionButton: FloatingActionButton(
        heroTag: 'queue',
        onPressed: () => showQueueBottomSheet(context),
        child: const Icon(Icons.queue_music),
      ),
      // The header takes the height it needs and the player gets the rest,
      // so they never overlap whatever the shape of the window.
      body: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(
                AppTheme.spacingMd,
                AppTheme.spacingMd,
                AppTheme.spacingMd,
                0,
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(
                    children: [
                      IconButton(
                        tooltip: 'Leave room',
                        icon: const Icon(Icons.exit_to_app_rounded),
                        onPressed: () =>
                            context.read<RoomController>().leaveRoom(),
                        style: IconButton.styleFrom(
                          backgroundColor: theme.colorScheme.errorContainer,
                          foregroundColor: theme.colorScheme.onErrorContainer,
                        ),
                      ),
                      Expanded(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(
                            horizontal: AppTheme.spacingMd,
                          ),
                          child: Text(
                            roomName,
                            style: theme.textTheme.headlineSmall?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                            textAlign: TextAlign.center,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ),
                      IconButton(
                        tooltip: 'Listeners',
                        icon: const Icon(Icons.people_rounded),
                        onPressed: () => showListenersDialog(context),
                        style: IconButton.styleFrom(
                          backgroundColor: theme.colorScheme.primary,
                          foregroundColor: theme.colorScheme.onPrimary,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: AppTheme.spacingSm),
                  const _ConnectionBanner(),
                ],
              ),
            ),
            const Expanded(child: PlayerWidget()),
          ],
        ),
      ),
    );
  }
}

/// Tells the user when the live connection to the room is not established.
class _ConnectionBanner extends StatelessWidget {
  const _ConnectionBanner();

  @override
  Widget build(BuildContext context) {
    final state = context.select<RoomController, WsConnectionState>(
      (c) => c.connectionState,
    );
    if (state == WsConnectionState.connected) return const SizedBox.shrink();

    final theme = Theme.of(context);
    final label = switch (state) {
      WsConnectionState.connecting => 'Connecting…',
      WsConnectionState.reconnecting => 'Connection lost. Reconnecting…',
      _ => 'Disconnected',
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.secondaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          SizedBox(
            width: 12,
            height: 12,
            child: CircularProgressIndicator(
              strokeWidth: 2,
              color: theme.colorScheme.onSecondaryContainer,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            label,
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSecondaryContainer,
            ),
          ),
        ],
      ),
    );
  }
}
