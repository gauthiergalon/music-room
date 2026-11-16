import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/friends_controller.dart';
import '../core/utils/ui_utils.dart';
import '../widgets/user_profile_dialog.dart';
import 'package:mobile/core/theme.dart';

class FriendsPage extends StatefulWidget {
  const FriendsPage({super.key});

  @override
  State<FriendsPage> createState() => _FriendsPageState();
}

class _FriendsPageState extends State<FriendsPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _fetchFriends();
    });
  }

  Future<void> _fetchFriends() async {
    try {
      await context.read<FriendsController>().fetchFriends();
    } catch (e) {
      if (mounted) UiUtils.showError(context, e.toString());
    }
  }

  Future<void> _showAddFriendDialog() async {
    final username = await showDialog<String>(
      context: context,
      builder: (_) => const _AddFriendDialog(),
    );
    if (username == null || username.isEmpty || !mounted) return;

    await _runFriendAction(
      (friends) => friends.addFriend(username),
      'Request sent to $username',
    );
  }

  Future<void> _runFriendAction(
    Future<void> Function(FriendsController friends) action,
    String successMessage,
  ) async {
    try {
      await action(context.read<FriendsController>());
      if (mounted) {
        UiUtils.showSuccess(context, successMessage);
      }
    } catch (e) {
      if (mounted) UiUtils.showError(context, e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Friends'),
        actions: [
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: _showAddFriendDialog,
          ),
        ],
      ),
      body: Consumer<FriendsController>(
        builder: (context, friendsController, child) {
          if (friendsController.isLoading) {
            return const Center(child: CircularProgressIndicator());
          }

          final pendingRequests = friendsController.pendingRequests;
          final acceptedFriends = friendsController.acceptedFriends;

          return RefreshIndicator(
            onRefresh: _fetchFriends,
            child: ListView(
              children: [
                if (pendingRequests.isNotEmpty) ...[
                  const Padding(
                    padding: AppTheme.screenPadding,
                    child: Text(
                      'Friend Requests',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ),
                  ...pendingRequests.map((friend) {
                    final friendId = friend.friendId;
                    return Card(
                      margin: AppTheme.paddingMd,
                      elevation: 1,
                      surfaceTintColor: Theme.of(
                        context,
                      ).colorScheme.surfaceTint,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: ListTile(
                        title: Text(
                          friend.username ?? 'Unknown User',
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                        subtitle: const Text('Wants to connect'),
                        onTap: () => showUserProfileDialog(
                          context,
                          friendId,
                          initialUsername: friend.username,
                        ),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            IconButton(
                              icon: const Icon(
                                Icons.check_circle,
                                color: Colors.green,
                              ),
                              onPressed: () => _runFriendAction(
                                (friends) => friends.acceptRequest(friendId),
                                'Friend request accepted',
                              ),
                            ),
                            IconButton(
                              icon: const Icon(
                                Icons.cancel,
                                color: Colors.grey,
                              ),
                              onPressed: () => _runFriendAction(
                                (friends) => friends.rejectRequest(friendId),
                                'Friend request rejected',
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  }),
                  const Divider(height: 32),
                ],
                const Padding(padding: AppTheme.screenPadding),
                if (acceptedFriends.isEmpty)
                  const Padding(
                    padding: AppTheme.paddingMd,
                    child: Center(
                      child: Text(
                        "You don't have any friends yet.",
                        style: TextStyle(color: Colors.grey, fontSize: 16),
                      ),
                    ),
                  )
                else
                  ...acceptedFriends.map((friend) {
                    final friendId = friend.friendId;
                    return Card(
                      elevation: 1,
                      margin: AppTheme.paddingMd,
                      surfaceTintColor: Theme.of(
                        context,
                      ).colorScheme.surfaceTint,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: ListTile(
                        title: Text(friend.username ?? 'Unknown User'),
                        onTap: () => showUserProfileDialog(
                          context,
                          friendId,
                          initialUsername: friend.username,
                        ),
                        trailing: IconButton(
                          icon: const Icon(
                            Icons.person_remove,
                            color: Colors.redAccent,
                          ),
                          onPressed: () => _runFriendAction(
                            (friends) => friends.removeFriend(friendId),
                            'Friend removed',
                          ),
                        ),
                      ),
                    );
                  }),
              ],
            ),
          );
        },
      ),
    );
  }
}

/// Asks for a username; pops with it, or with nothing when cancelled.
class _AddFriendDialog extends StatefulWidget {
  const _AddFriendDialog();

  @override
  State<_AddFriendDialog> createState() => _AddFriendDialogState();
}

class _AddFriendDialogState extends State<_AddFriendDialog> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final username = _controller.text.trim();
    if (username.isEmpty) return;
    Navigator.pop(context, username);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
      title: const Text('Add Friend'),
      actionsPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      content: TextField(
        controller: _controller,
        onSubmitted: (_) => _submit(),
        decoration: const InputDecoration(
          labelText: 'Username',
          hintText: 'Enter username',
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        ElevatedButton(onPressed: _submit, child: const Text('Add')),
      ],
    );
  }
}
