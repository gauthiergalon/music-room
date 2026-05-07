import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/friends_controller.dart';
import '../controllers/room_controller.dart';
import '../controllers/auth_controller.dart';
import '../core/theme.dart';
import '../models/room_user.dart';

Future<void> _showInviteFriendDialog(
  BuildContext context,
  String roomId,
  List<RoomUser> listeners,
) async {
  final authController = context.read<AuthController>();
  final friendsController = context.read<FriendsController>();
  final currentUser = authController.user;

  if (currentUser == null) {
    return;
  }

  try {
    await friendsController.fetchFriends(currentUser.id);
  } catch (e) {
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(e.toString()),
          backgroundColor: Colors.red,
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
    return;
  }

  if (!context.mounted) {
    return;
  }

  final listenerIds = listeners.map((listener) => listener.id).toSet();
  final availableFriends = friendsController.acceptedFriends
      .where(
        (friend) =>
            friend.friendId != currentUser.id &&
            !listenerIds.contains(friend.friendId),
      )
      .toList();

  await showDialog<void>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Invite a friend'),
      content: SizedBox(
        width: double.maxFinite,
        child: availableFriends.isEmpty
            ? const Text('No friends available to invite.')
            : ListView.separated(
                shrinkWrap: true,
                itemCount: availableFriends.length,
                separatorBuilder: (_, __) => const Divider(height: 1),
                itemBuilder: (itemContext, index) {
                  final friend = availableFriends[index];
                  return ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading: const Icon(Icons.person),
                    title: Text(friend.username ?? 'Unknown User'),
                    onTap: () async {
                      Navigator.of(dialogContext).pop();
                      try {
                        await friendsController.inviteToRoom(
                          roomId,
                          friend.friendId,
                        );
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(
                                'Invitation sent to ${friend.username ?? 'friend'}',
                              ),
                              behavior: SnackBarBehavior.floating,
                            ),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(
                              content: Text(e.toString()),
                              backgroundColor: Colors.red,
                              behavior: SnackBarBehavior.floating,
                            ),
                          );
                        }
                      }
                    },
                  );
                },
              ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(),
          child: const Text('Close'),
        ),
      ],
    ),
  );
}

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
                                : 'Private (friends invitations only)',
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
                                    behavior: SnackBarBehavior.floating,
                                  ),
                                );
                              }
                            }
                          },
                        ),
                        SwitchListTile(
                          title: const Text('Room License'),
                          subtitle: Text(
                            currentRoom.isLicensed
                                ? 'Licensed (Invited users can edit queue)'
                                : 'Unlicensed (Everyone can edit queue)',
                          ),
                          value: currentRoom.isLicensed,
                          onChanged: (val) async {
                            try {
                              await controller.toggleLicense(currentRoom);
                            } catch (e) {
                              if (context.mounted) {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(
                                    content: Text(
                                      'Failed to update room license',
                                    ),
                                    backgroundColor: Colors.red,
                                    behavior: SnackBarBehavior.floating,
                                  ),
                                );
                              }
                            }
                          },
                        ),
                        ListTile(
                          leading: const Icon(Icons.person_add),
                          title: const Text('Invite a friend'),
                          subtitle: const Text('Choose from your friends list'),
                          onTap: () => _showInviteFriendDialog(
                            context,
                            currentRoom.id,
                            otherListeners,
                          ),
                        ),
                        const Divider(),
                      ],
                      const Padding(
                        padding: EdgeInsets.symmetric(
                          horizontal: 16,
                          vertical: 8,
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
                                            Icons.card_membership,
                                            size: 20,
                                          ),
                                          onPressed: null,
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
