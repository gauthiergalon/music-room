import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/auth_controller.dart';
import '../controllers/invitations_controller.dart';
import '../controllers/room_controller.dart';
import '../models/room.dart';
import '../models/room_invitation.dart';
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

  late final RoomController _roomController;
  bool _isCreating = false;

  @override
  void initState() {
    super.initState();
    _roomController = context.read<RoomController>();
    _roomController.addListener(_showRoomMessages);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refreshRooms();
    });
  }

  @override
  void dispose() {
    _roomController.removeListener(_showRoomMessages);
    super.dispose();
  }

  /// Shows the one-off messages of the room controller (room closed, player
  /// error) as snackbars.
  void _showRoomMessages() {
    if (!_roomController.hasRoomClosedMessage &&
        !_roomController.hasPlayerErrorMessage) {
      return;
    }
    // The controller may notify while the tree is being built, so wait for
    // the end of the frame (and make sure there is one).
    WidgetsBinding.instance.ensureVisualUpdate();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final closedMessage = _roomController.takeRoomClosedMessage();
      if (closedMessage != null) UiUtils.showSnackBar(context, closedMessage);
      final errorMessage = _roomController.takePlayerErrorMessage();
      if (errorMessage != null) UiUtils.showError(context, errorMessage);
    });
  }

  Future<void> _createRoom() async {
    if (_isCreating) return;

    setState(() {
      _isCreating = true;
    });

    try {
      await _roomController.createRoom();
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
        _roomController.refreshRooms(),
        context.read<InvitationsController>().fetch(),
      ]).timeout(const Duration(seconds: 10));
    } catch (e) {
      if (mounted) {
        UiUtils.showError(context, 'Failed to refresh rooms. Try again later.');
      }
    }
  }

  Future<void> _fetchInvitations({required bool showErrors}) async {
    try {
      await context.read<InvitationsController>().fetch();
    } catch (e) {
      if (showErrors && mounted) {
        UiUtils.showError(context, 'Failed to load invitations');
      }
    }
  }

  Future<void> _answerInvitation(
    RoomInvitation invitation, {
    required bool accept,
  }) async {
    final invitations = context.read<InvitationsController>();
    if (invitations.isProcessing(invitation.id)) return;

    try {
      if (accept) {
        await invitations.accept(invitation);
      } else {
        await invitations.reject(invitation);
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
      return;
    }

    if (!mounted) return;
    UiUtils.showSuccess(
      context,
      accept ? 'Invitation accepted' : 'Invitation rejected',
    );
    if (!accept) return;

    // Join the room the invitation was for.
    try {
      await _roomController.refreshRooms();
      final room = _roomController.availableRooms
          .where((room) => room.id == invitation.roomId)
          .firstOrNull;
      if (room != null) await _roomController.openRoom(room);
    } catch (_) {
      // The invitation is accepted; the room can still be joined from the
      // list once it loads.
    }
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

    final inRoom = context.select<RoomController, bool>(
      (c) => c.currentRoom != null,
    );
    final hasInvitations = context.select<InvitationsController, bool>(
      (c) => c.pending.isNotEmpty,
    );

    return PopScope(
      canPop: !inRoom,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop) return;
        _roomController.leaveRoom();
      },
      child: Scaffold(
        key: _scaffoldKey,
        appBar: inRoom
            ? null
            : AppBar(
                actions: [
                  IconButton(
                    icon: const Icon(Icons.refresh),
                    tooltip: 'Refresh rooms',
                    onPressed: _refreshRooms,
                  ),
                  Stack(
                    clipBehavior: Clip.none,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.notifications_none),
                        tooltip: 'Room invitations',
                        onPressed: () {
                          _fetchInvitations(showErrors: false);
                          _scaffoldKey.currentState?.openEndDrawer();
                        },
                      ),
                      if (hasInvitations)
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
              ),
        endDrawer: _InvitationsDrawer(
          onRefresh: () => _fetchInvitations(showErrors: true),
          onAnswer: _answerInvitation,
        ),
        floatingActionButton: inRoom
            ? null
            : FloatingActionButton(
                onPressed: _isCreating ? null : _createRoom,
                child: _isCreating
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.add),
              ),
        body: SafeArea(
          bottom: false,
          child: inRoom
              ? const RoomOverlay()
              : _RoomList(onRefresh: _refreshRooms),
        ),
      ),
    );
  }
}

class _RoomList extends StatelessWidget {
  const _RoomList({required this.onRefresh});

  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    final rooms = context.select<RoomController, List<Room>>(
      (c) => c.availableRooms,
    );

    if (rooms.isEmpty) {
      return RefreshIndicator(
        onRefresh: onRefresh,
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
                    const Text(
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
      onRefresh: onRefresh,
      child: ListView.builder(
        padding: const EdgeInsets.symmetric(
          vertical: AppTheme.spacingLg,
          horizontal: AppTheme.spacingSm,
        ),
        physics: const AlwaysScrollableScrollPhysics(),
        itemCount: rooms.length,
        itemBuilder: (context, index) => RoomListItem(
          room: rooms[index],
          onTap: () => context.read<RoomController>().openRoom(rooms[index]),
        ),
      ),
    );
  }
}

class _InvitationsDrawer extends StatelessWidget {
  const _InvitationsDrawer({required this.onRefresh, required this.onAnswer});

  final VoidCallback onRefresh;
  final Future<void> Function(RoomInvitation invitation, {required bool accept})
  onAnswer;

  @override
  Widget build(BuildContext context) {
    final invitations = context.watch<InvitationsController>();
    final pending = invitations.pending;

    return Drawer(
      child: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: AppTheme.paddingMd,
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
                    onPressed: invitations.isLoading ? null : onRefresh,
                    icon: const Icon(Icons.refresh),
                    tooltip: 'Refresh',
                  ),
                ],
              ),
            ),
            Expanded(
              child: invitations.isLoading
                  ? const Center(child: CircularProgressIndicator())
                  : pending.isEmpty
                  ? const Center(
                      child: Padding(
                        padding: AppTheme.paddingMd,
                        child: Text(
                          'No pending invitations',
                          textAlign: TextAlign.center,
                          style: TextStyle(color: Colors.grey),
                        ),
                      ),
                    )
                  : ListView.separated(
                      padding: AppTheme.paddingMd,
                      itemCount: pending.length,
                      separatorBuilder: (_, index) =>
                          const SizedBox(height: 10),
                      itemBuilder: (context, index) {
                        final invitation = pending[index];
                        return _InvitationCard(
                          invitation: invitation,
                          isProcessing: invitations.isProcessing(invitation.id),
                          onAnswer: onAnswer,
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}

class _InvitationCard extends StatelessWidget {
  const _InvitationCard({
    required this.invitation,
    required this.isProcessing,
    required this.onAnswer,
  });

  final RoomInvitation invitation;
  final bool isProcessing;
  final Future<void> Function(RoomInvitation invitation, {required bool accept})
  onAnswer;

  @override
  Widget build(BuildContext context) {
    final roomName = invitation.roomName;

    return Card(
      child: Padding(
        padding: AppTheme.paddingMd,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              invitation.inviterUsername,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 4),
            Text(
              roomName == null || roomName.isEmpty
                  ? 'invited you to join a room'
                  : 'invited you to join "$roomName"',
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: ElevatedButton.icon(
                    onPressed: isProcessing
                        ? null
                        : () => onAnswer(invitation, accept: true),
                    icon: const Icon(Icons.check),
                    label: const Text('Accept'),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: OutlinedButton.icon(
                    onPressed: isProcessing
                        ? null
                        : () => onAnswer(invitation, accept: false),
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
  }
}
