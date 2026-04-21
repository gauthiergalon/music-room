import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/auth_controller.dart';
import '../controllers/friends_controller.dart';
import '../core/utils/ui_utils.dart';

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
    final authController = context.read<AuthController>();
    final friendsController = context.read<FriendsController>();
    try {
      await friendsController.fetchFriends(authController.user?.id);
    } catch (e) {
      if (mounted) UiUtils.showError(context, e.toString());
    }
  }

  void _showAddFriendDialog() {
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Add Friend'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            labelText: 'Username',
            hintText: 'Enter username',
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              if (controller.text.isEmpty) return;
              final username = controller.text;
              Navigator.pop(ctx);

              final authController = context.read<AuthController>();
              final friendsController = context.read<FriendsController>();

              try {
                await friendsController.addFriend(
                  username,
                  authController.user?.id,
                );
                if (mounted) {
                  UiUtils.showSuccess(context, 'Request sent to $username');
                }
              } catch (e) {
                if (mounted) UiUtils.showError(context, e.toString());
              }
            },
            child: const Text('Add'),
          ),
        ],
      ),
    );
  }

  Future<void> _handleFriendAction(
    String endpoint,
    String successMessage,
  ) async {
    final authController = context.read<AuthController>();
    final friendsController = context.read<FriendsController>();

    try {
      await friendsController.handleAction(endpoint, authController.user?.id);
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
                    padding: EdgeInsets.all(16.0),
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
                      margin: const EdgeInsets.symmetric(
                        horizontal: 16.0,
                        vertical: 4.0,
                      ),
                      child: ListTile(
                        title: Text(
                          friend.username ?? 'Unknown User',
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                        subtitle: const Text('Wants to connect'),
                        trailing: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            IconButton(
                              icon: const Icon(
                                Icons.check_circle,
                                color: Colors.green,
                              ),
                              onPressed: () => _handleFriendAction(
                                '/friends/$friendId/accept',
                                'Friend request accepted',
                              ),
                            ),
                            IconButton(
                              icon: const Icon(
                                Icons.cancel,
                                color: Colors.grey,
                              ),
                              onPressed: () => _handleFriendAction(
                                '/friends/$friendId/reject',
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
                const Padding(
                  padding: EdgeInsets.all(16.0),
                  child: Text(
                    'My Friends',
                    style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
                  ),
                ),
                if (acceptedFriends.isEmpty)
                  const Padding(
                    padding: EdgeInsets.all(32.0),
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
                      elevation: 0,
                      margin: const EdgeInsets.symmetric(
                        horizontal: 16.0,
                        vertical: 2.0,
                      ),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(8.0),
                      ),
                      child: ListTile(
                        title: Text(friend.username ?? 'Unknown User'),
                        trailing: IconButton(
                          icon: const Icon(
                            Icons.person_remove,
                            color: Colors.redAccent,
                          ),
                          onPressed: () => _handleFriendAction(
                            '/friends/$friendId',
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
