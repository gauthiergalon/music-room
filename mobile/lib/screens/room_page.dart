import 'dart:math';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../controllers/auth_controller.dart';
import '../controllers/room_controller.dart';
import '../models/room.dart';
import '../widgets/room_list_item.dart';
import '../core/theme.dart';
import 'room_overlay.dart';

class RoomPage extends StatefulWidget {
  const RoomPage({super.key});

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  void _createRoom(String username) {
    final newRoom = Room(
      id: Random().nextInt(1000000),
      owner: username,
      currentTrack: null,
      status: 0,
      listeners: [username],
    );
    final controller = context.read<RoomController>();
    controller.createRoom(newRoom);
    controller.openRoom(newRoom);
  }

  void _openRoom(Room room) {
    context.read<RoomController>().openRoom(room);
  }

  @override
  Widget build(BuildContext context) {
    final authController = context.watch<AuthController>();
    final user = authController.user;

    if (user == null) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
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
                onPressed: () => _createRoom(user.username),
                child: const Icon(Icons.add),
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
    return RefreshIndicator(
      onRefresh: controller.refreshRooms,
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
