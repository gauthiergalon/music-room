import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/auth_controller.dart';
import '../controllers/room_controller.dart';
import '../core/network/api_client.dart';
import '../models/room.dart';
import '../widgets/room_list_item.dart';
import '../core/theme.dart';
import '../core/utils/ui_utils.dart';
import 'room_overlay.dart';

class RoomPage extends StatefulWidget {
  const RoomPage({super.key});

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();

  bool _isCreating = false;
  bool _isLoadingInvitations = false;
  final Set<String> _processingInvitationIds = <String>{};
  List<_RoomInvitation> _pendingInvitations = <_RoomInvitation>[];

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _fetchPendingInvitations(showErrors: false);
    });
  }

  Future<void> _createRoom(String username) async {
    if (_isCreating) return;

    setState(() {
      _isCreating = true;
    });

    final controller = context.read<RoomController>();
    try {
      final room = await controller.createRoom();
      controller.openRoom(room);
    } catch (e) {
      if (mounted) {
        UiUtils.showError(context, e.toString());
      }
    } finally {
      if (mounted) {
        setState(() {
          _isCreating = false;
        });
      }
    }
  }

  Future<void> _refreshRooms() async {
    try {
      await Future.wait([
        context.read<RoomController>().refreshRooms(),
        _fetchPendingInvitations(showErrors: false),
      ]);
    } catch (e) {
      if (mounted) {
        UiUtils.showError(context, e.toString());
      }
    }
  }

  Future<void> _fetchPendingInvitations({bool showErrors = true}) async {
    if (mounted) {
      setState(() {
        _isLoadingInvitations = true;
      });
    }

    try {
      final response = await ApiClient.get('/me/invitations');
      final rawList = List<dynamic>.from(response as List<dynamic>);
      final invitations = rawList
          .map(
            (item) => _RoomInvitation.fromJson(
              Map<String, dynamic>.from(item as Map),
            ),
          )
          .where((inv) => inv.isPending)
          .toList();

      final inviterIds = invitations
          .map((inv) => inv.inviterId)
          .where((id) => id.isNotEmpty)
          .toSet();

      final Map<String, String> usernamesById = <String, String>{};
      await Future.wait(
        inviterIds.map((inviterId) async {
          try {
            final userRes = await ApiClient.get('/users/$inviterId');
            final username = userRes['username']?.toString();
            if (username != null && username.isNotEmpty) {
              usernamesById[inviterId] = username;
            }
          } catch (_) {
            // Keep fallback username when user details are unavailable.
          }
        }),
      );

      if (!mounted) return;
      setState(() {
        _pendingInvitations = invitations
            .map(
              (inv) => inv.copyWith(
                inviterUsername: usernamesById[inv.inviterId] ?? 'Unknown user',
              ),
            )
            .toList();
      });
    } catch (e) {
      if (showErrors && mounted) {
        UiUtils.showError(context, 'Failed to load invitations');
      }
    } finally {
      if (mounted) {
        setState(() {
          _isLoadingInvitations = false;
        });
      }
    }
  }

  Future<void> _handleInvitationAction(
    _RoomInvitation invitation, {
    required bool accept,
  }) async {
    if (_processingInvitationIds.contains(invitation.id)) {
      return;
    }

    setState(() {
      _processingInvitationIds.add(invitation.id);
    });

    try {
      final endpoint = accept
          ? '/me/invitations/${invitation.id}/accept'
          : '/me/invitations/${invitation.id}/reject';
      await ApiClient.post(endpoint);

      if (!mounted) return;

      setState(() {
        _pendingInvitations.removeWhere((i) => i.id == invitation.id);
      });

      await context.read<RoomController>().refreshRooms();

      if (mounted) {
        UiUtils.showSuccess(
          context,
          accept ? 'Invitation accepted' : 'Invitation rejected',
        );
      }
    } catch (e) {
      if (mounted) {
        UiUtils.showError(
          context,
          accept
              ? 'Failed to accept invitation'
              : 'Failed to reject invitation',
        );
      }
    } finally {
      if (mounted) {
        setState(() {
          _processingInvitationIds.remove(invitation.id);
        });
      }
    }
  }

  void _openRoom(Room room) {
    context.read<RoomController>().openRoom(room);
  }

  @override
  Widget build(BuildContext context) {
    final authController = context.watch<AuthController>();
    final user = authController.user;

    if (user == null) {
      if (authController.isLoadingUser) {
        return const Scaffold(body: Center(child: CircularProgressIndicator()));
      }
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.error_outline,
                size: 48,
                color: Colors.redAccent,
              ),
              const SizedBox(height: 16),
              const Text('An error occurred while loading data.'),
              const SizedBox(height: 16),
              ElevatedButton.icon(
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
                onPressed: () => authController.fetchUserInfo(),
              ),
            ],
          ),
        ),
      );
    }

    final controller = context.watch<RoomController>();
    final current = controller.currentRoom;

    return PopScope(
      canPop: current == null,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop) return;
        if (current != null) {
          controller.leaveRoom();
        }
      },
      child: Scaffold(
        key: _scaffoldKey,
        appBar: current == null
            ? AppBar(
                actions: [
                  Stack(
                    clipBehavior: Clip.none,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.notifications_none),
                        tooltip: 'Room invitations',
                        onPressed: () {
                          _fetchPendingInvitations(showErrors: false);
                          _scaffoldKey.currentState?.openEndDrawer();
                        },
                      ),
                      if (_pendingInvitations.isNotEmpty)
                        Positioned(
                          right: 12,
                          top: 12,
                          child: Container(
                            width: 10,
                            height: 10,
                            decoration: BoxDecoration(
                              color: Theme.of(context).colorScheme.secondary,
                              borderRadius: BorderRadius.circular(999),
                            ),
                          ),
                        ),
                    ],
                  ),
                ],
              )
            : null,
        endDrawer: Drawer(
          child: SafeArea(
            child: Column(
              children: [
                Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Row(
                    children: [
                      const Expanded(
                        child: Text(
                          'Room invitations',
                          style: TextStyle(
                            fontSize: 20,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ),
                      IconButton(
                        onPressed: _isLoadingInvitations
                            ? null
                            : () => _fetchPendingInvitations(showErrors: true),
                        icon: const Icon(Icons.refresh),
                        tooltip: 'Refresh',
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: _isLoadingInvitations
                      ? const Center(child: CircularProgressIndicator())
                      : _pendingInvitations.isEmpty
                      ? const Center(
                          child: Padding(
                            padding: EdgeInsets.symmetric(horizontal: 24.0),
                            child: Text(
                              'No pending invitations',
                              textAlign: TextAlign.center,
                              style: TextStyle(color: Colors.grey),
                            ),
                          ),
                        )
                      : ListView.separated(
                          padding: const EdgeInsets.all(12),
                          itemCount: _pendingInvitations.length,
                          separatorBuilder: (_, __) =>
                              const SizedBox(height: 10),
                          itemBuilder: (context, index) {
                            final invitation = _pendingInvitations[index];
                            final isProcessing = _processingInvitationIds
                                .contains(invitation.id);

                            return Card(
                              child: Padding(
                                padding: const EdgeInsets.all(12.0),
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      invitation.inviterUsername,
                                      style: const TextStyle(
                                        fontSize: 16,
                                        fontWeight: FontWeight.w600,
                                      ),
                                    ),
                                    const SizedBox(height: 4),
                                    const Text('invited you to join a room'),
                                    const SizedBox(height: 12),
                                    Row(
                                      children: [
                                        Expanded(
                                          child: ElevatedButton.icon(
                                            onPressed: isProcessing
                                                ? null
                                                : () => _handleInvitationAction(
                                                    invitation,
                                                    accept: true,
                                                  ),
                                            icon: const Icon(Icons.check),
                                            label: const Text('Accept'),
                                          ),
                                        ),
                                        const SizedBox(width: 8),
                                        Expanded(
                                          child: OutlinedButton.icon(
                                            onPressed: isProcessing
                                                ? null
                                                : () => _handleInvitationAction(
                                                    invitation,
                                                    accept: false,
                                                  ),
                                            icon: const Icon(Icons.close),
                                            label: const Text('Reject'),
                                          ),
                                        ),
                                      ],
                                    ),
                                  ],
                                ),
                              ),
                            );
                          },
                        ),
                ),
              ],
            ),
          ),
        ),
        floatingActionButton: current == null
            ? FloatingActionButton(
                onPressed: _isCreating
                    ? null
                    : () => _createRoom(user.username),
                child: _isCreating
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.add),
              )
            : null,
        body: SafeArea(
          bottom: false,
          child: Stack(
            children: [
              _buildRoomList(controller, user.username),
              if (current != null) const Positioned.fill(child: RoomOverlay()),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildRoomList(RoomController controller, String username) {
    final rooms = controller.availableRooms.where((r) => r.isPublic).toList();

    if (rooms.isEmpty) {
      return RefreshIndicator(
        onRefresh: _refreshRooms,
        child: CustomScrollView(
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: [
            SliverFillRemaining(
              hasScrollBody: false,
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.music_off_outlined,
                      size: 64,
                      color: Theme.of(context).colorScheme.secondary,
                    ),
                    const SizedBox(height: 16),
                    Text(
                      'No rooms available',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Be the first to create one!',
                      style: TextStyle(
                        fontSize: 14,
                        color: Colors.grey.shade500,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      );
    }

    return RefreshIndicator(
      onRefresh: _refreshRooms,
      child: ListView.builder(
        padding: const EdgeInsets.only(top: 10.0, bottom: AppTheme.spacingMd),
        physics: const AlwaysScrollableScrollPhysics(),
        itemCount: rooms.length,
        itemBuilder: (context, index) => RoomListItem(
          room: rooms[index],
          onTap: () => _openRoom(rooms[index]),
        ),
      ),
    );
  }
}

class _RoomInvitation {
  final String id;
  final String roomId;
  final String inviterId;
  final bool isPending;
  final String inviterUsername;

  const _RoomInvitation({
    required this.id,
    required this.roomId,
    required this.inviterId,
    required this.isPending,
    required this.inviterUsername,
  });

  factory _RoomInvitation.fromJson(Map<String, dynamic> json) {
    return _RoomInvitation(
      id: json['id']?.toString() ?? '',
      roomId: json['room_id']?.toString() ?? '',
      inviterId: json['inviter_id']?.toString() ?? '',
      isPending: json['is_pending'] == true,
      inviterUsername: 'Unknown user',
    );
  }

  _RoomInvitation copyWith({String? inviterUsername}) {
    return _RoomInvitation(
      id: id,
      roomId: roomId,
      inviterId: inviterId,
      isPending: isPending,
      inviterUsername: inviterUsername ?? this.inviterUsername,
    );
  }
}
