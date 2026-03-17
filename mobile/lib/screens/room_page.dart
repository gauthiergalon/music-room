import 'dart:math';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
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
  final String currentUser = 'You';

  void _createRoom() {
    final newRoom = Room(
      id: Random().nextInt(1000000),
      owner: 'You',
      currentTrack: null,
      status: 0,
      listeners: ['You'],
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
                onPressed: _createRoom,
                child: const Icon(Icons.add),
              )
            : null,
        body: SafeArea(
          bottom: false,
          child: Stack(
            children: [
              _buildRoomList(controller),
              if (current != null) const Positioned.fill(child: RoomOverlay()),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildRoomList(RoomController controller) {
    final rooms = controller.availableRooms
        .where((r) => r.isPublic || r.listeners.contains(currentUser))
        .toList();
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
