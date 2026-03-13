import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'controllers/room_controller.dart';
import 'models/room.dart';

void main() {
  runApp(
    ChangeNotifierProvider(
      create: (_) => RoomController(),
      child: const MyApp(),
    ),
  );
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Music Room',
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.deepPurple,
          brightness: Brightness.dark,
        ),
      ),
      home: const MainScreen(),
    );
  }
}

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  State<MainScreen> createState() => _MainScreenState();
}

class _MainScreenState extends State<MainScreen> {
  int _selectedIndex = 0;

  @override
  Widget build(BuildContext context) {
    final pages = <Widget>[
      const RoomPage(),
      const SearchPage(),
      const ProfilePage(),
    ];

    return Scaffold(
      body: pages[_selectedIndex],
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (int index) {
          setState(() {
            _selectedIndex = index;
          });
        },
        destinations: const <NavigationDestination>[
          NavigationDestination(
            icon: Icon(Icons.group_outlined),
            label: 'Room',
          ),
          NavigationDestination(
            icon: Icon(Icons.search_outlined),
            label: 'Search',
          ),
          NavigationDestination(
            icon: Icon(Icons.person_outline),
            label: 'Profile',
          ),
        ],
        backgroundColor: Theme.of(context).colorScheme.surface,
      ),
    );
  }
}

class RoomPage extends StatefulWidget {
  const RoomPage({super.key});

  @override
  State<RoomPage> createState() => _RoomPageState();
}

class _RoomPageState extends State<RoomPage> {
  void _createRoom() {
    final newRoom = Room(owner: 'You', currentTrack: '', status: 0, people: 1);
    context.read<RoomController>().createRoom(newRoom);
    context.read<RoomController>().openRoom(newRoom);
  }

  void _openRoom(Room room) {
    context.read<RoomController>().openRoom(room);
  }

  void _leaveRoom() {
    context.read<RoomController>().leaveRoom();
  }

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<RoomController>();
    final current = controller.currentRoom;

    return Stack(
      children: [
        if (current == null) _buildRoomList(controller),
        if (current != null) _buildRoomDetail(controller),

        Positioned(
          right: 16,
          bottom: 16,
          child: FloatingActionButton(
            onPressed: current == null ? _createRoom : null,
            child: current == null
                ? const Icon(Icons.add)
                : const Icon(Icons.queue),
          ),
        ),
      ],
    );
  }

  Widget _buildRoomList(RoomController controller) {
    final rooms = controller.availableRooms;
    return ListView.builder(
      itemCount: rooms.length,
      itemBuilder: (context, index) => _roomItem(rooms[index]),
    );
  }

  Widget _roomItem(Room room) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: FilledButton.tonal(
        onPressed: () => _openRoom(room),
        style: FilledButton.styleFrom(
          minimumSize: const Size(double.infinity, 72),
          alignment: Alignment.centerLeft,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        ),
        child: Row(
          children: [
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    room.owner,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      const Icon(Icons.music_note, size: 16),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          room.currentTrack.isNotEmpty
                              ? room.currentTrack
                              : 'No track',
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            const Icon(Icons.people),
            const SizedBox(width: 4),
            Text(room.people.toString()),
          ],
        ),
      ),
    );
  }

  Widget _buildRoomDetail(RoomController controller) {
    final room = controller.currentRoom!;
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Text(room.owner, style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 12),
          Text(
            room.currentTrack.isNotEmpty
                ? room.currentTrack
                : 'No track playing',
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          FilledButton.tonal(
            onPressed: _leaveRoom,
            child: const Text('Leave room'),
          ),
        ],
      ),
    );
  }
}

class SearchPage extends StatelessWidget {
  const SearchPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(child: Text('Search'));
  }
}

class ProfilePage extends StatelessWidget {
  const ProfilePage({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(child: Text('Profile'));
  }
}
