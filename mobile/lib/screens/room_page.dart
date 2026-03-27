import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/auth_controller.dart';
import '../controllers/room_controller.dart';
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
  bool _isCreating = false;

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
      await context.read<RoomController>().refreshRooms();
    } catch (e) {
      if (mounted) {
        UiUtils.showError(context, e.toString());
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
