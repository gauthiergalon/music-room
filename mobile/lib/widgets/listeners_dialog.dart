import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../core/theme.dart';

void showListenersDialog(BuildContext context, String currentUser) {
  showGeneralDialog(
    context: context,
    barrierLabel: 'Listeners',
    barrierDismissible: true,
    barrierColor: Colors.black54,
    transitionDuration: const Duration(milliseconds: 300),
    pageBuilder: (ctx, anim1, anim2) {
      final size = MediaQuery.of(ctx).size;
      return Align(
        alignment: Alignment.centerRight,
        child: Material(
          color: Theme.of(ctx).colorScheme.surface,
          elevation: 8,
          shape: const RoundedRectangleBorder(
            borderRadius: BorderRadius.only(
              topLeft: Radius.circular(12),
              bottomLeft: Radius.circular(12),
            ),
          ),
          child: SizedBox(
            width: size.width * 0.78,
            height: size.height,
            child: SafeArea(
              child: Consumer<RoomController>(
                builder: (ctx2, controller, _) {
                  final currentRoom = controller.currentRoom;
                  if (currentRoom == null) return const SizedBox.shrink();

                  final otherListeners = currentRoom.listeners
                      .where((l) => l != currentRoom.owner)
                      .toList();

                  return Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: AppTheme.spacingMd,
                          vertical: 12,
                        ),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            const Text(
                              'Listeners',
                              style: TextStyle(
                                fontSize: 18,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                            IconButton(
                              icon: const Icon(Icons.close),
                              onPressed: () => Navigator.of(ctx).pop(),
                            ),
                          ],
                        ),
                      ),
                      ListTile(
                        title: Row(
                          children: [
                            const Icon(Icons.emoji_events, size: 18),
                            const SizedBox(width: 8),
                            Text(
                              currentRoom.owner,
                              style: const TextStyle(
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ],
                        ),
                      ),
                      if (currentUser == currentRoom.owner) ...[
                        const Divider(),
                        SwitchListTile(
                          title: const Text('Room Publique'),
                          subtitle: Text(
                            currentRoom.isPublic
                                ? 'Visible par tous'
                                : 'Cachée (invitation uniquement)',
                          ),
                          value: currentRoom.isPublic,
                          onChanged: (val) {
                            controller.togglePrivacy(currentRoom);
                          },
                        ),
                        ListTile(
                          leading: const Icon(Icons.link),
                          title: const Text('Copier le lien d\'invitation'),
                          onTap: () {
                            Clipboard.setData(
                              ClipboardData(
                                text:
                                    'musicroom://join/${currentRoom.id.toString()}',
                              ),
                            );
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(
                                content: Text(
                                  'Lien copié dans le presse-papier !',
                                ),
                              ),
                            );
                          },
                        ),
                        const Divider(),
                      ],
                      const Padding(
                        padding: EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 8,
                        ),
                        child: Text(
                          'Autres auditeurs',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            color: Colors.grey,
                          ),
                        ),
                      ),
                      Expanded(
                        child: ListView.builder(
                          itemCount: otherListeners.length,
                          itemBuilder: (c, idx) {
                            final name = otherListeners[idx];
                            final amOwner = currentUser == currentRoom.owner;
                            return ListTile(
                              title: Text(name),
                              trailing: amOwner
                                  ? Row(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        IconButton(
                                          icon: const Icon(
                                            Icons.emoji_events,
                                            size: 20,
                                          ),
                                          onPressed: () =>
                                              controller.promoteToOwner(
                                                currentRoom,
                                                name,
                                              ),
                                        ),
                                        IconButton(
                                          icon: const Icon(
                                            Icons.remove_circle_outline,
                                          ),
                                          onPressed: () => controller
                                              .kickListener(currentRoom, name),
                                        ),
                                      ],
                                    )
                                  : null,
                            );
                          },
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      );
    },
    transitionBuilder: (ctx, anim, secAnim, child) {
      final tween = Tween(begin: const Offset(1, 0), end: Offset.zero);
      return SlideTransition(
        position: tween.animate(
          CurvedAnimation(parent: anim, curve: Curves.easeOut),
        ),
        child: child,
      );
    },
  );
}
