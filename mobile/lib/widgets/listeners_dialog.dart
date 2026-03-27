import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import '../controllers/room_controller.dart';
import '../controllers/auth_controller.dart';
import '../core/theme.dart';

void showListenersDialog(BuildContext context) {
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
              child: Consumer2<RoomController, AuthController>(
                builder: (ctx2, controller, authController, _) {
                  final currentRoom = controller.currentRoom;
                  final currentUser = authController.user;
                  if (currentRoom == null || currentUser == null) {
                    return const SizedBox.shrink();
                  }

                  final otherListeners = currentRoom.listeners.toList();

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
                      if (currentUser.id == currentRoom.owner) ...[
                        const Divider(),
                        SwitchListTile(
                          title: const Text('Room Privacy'),
                          subtitle: Text(
                            currentRoom.isPublic
                                ? 'Public'
                                : 'Private (invitation link only)',
                          ),
                          value: currentRoom.isPublic,
                          onChanged: (val) async {
                            try {
                              await controller.togglePrivacy(currentRoom);
                            } catch (e) {
                              if (context.mounted) {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text(
                                      'Failed to update room privacy',
                                    ),
                                    backgroundColor: Colors.red,
                                  ),
                                );
                              }
                            }
                          },
                        ),
                        ListTile(
                          leading: const Icon(Icons.link),
                          title: const Text('Copy Invitation Link'),
                          onTap: () {
                            Clipboard.setData(
                              ClipboardData(
                                text:
                                    'musicroom://join/${currentRoom.id.toString()}',
                              ),
                            );
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(
                                content: Text('Link copied to clipboard!'),
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
                          'Members of the room',
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
                            final roomUser = otherListeners[idx];
                            final amOwner = currentUser.id == currentRoom.owner;
                            final isMe = roomUser.id == currentUser.id;
                            final isOwnerUser =
                                roomUser.id == currentRoom.owner;

                            Widget? leadingIcon;
                            if (isOwnerUser) {
                              leadingIcon = const Icon(
                                Icons.emoji_events,
                                size: 18,
                                color: Colors.amber,
                              );
                            } else {
                              leadingIcon = const SizedBox(width: 18);
                            }

                            return ListTile(
                              leading: leadingIcon,
                              title: Text(
                                roomUser.username,
                                style: TextStyle(
                                  fontWeight: isMe
                                      ? FontWeight.bold
                                      : FontWeight.normal,
                                ),
                              ),
                              trailing: (amOwner && !isMe)
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
                                                roomUser,
                                              ),
                                        ),
                                        IconButton(
                                          icon: const Icon(
                                            Icons.remove_circle_outline,
                                            size: 20,
                                          ),
                                          onPressed: () =>
                                              controller.kickListener(
                                                currentRoom,
                                                roomUser,
                                              ),
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
