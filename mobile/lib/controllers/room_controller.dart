import 'package:flutter/foundation.dart';
import '../models/room.dart';

class RoomController extends ChangeNotifier {
  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;

  final List<Room> _availableRooms = [
    Room(
      owner: 'Alice',
      currentTrack: 'Song A - Artist X',
      status: 1,
      people: 3,
    ),
    Room(owner: 'Bob', currentTrack: 'Song B - Artist Y', status: 0, people: 2),
    Room(owner: 'Charlie', currentTrack: '', status: 0, people: 1),
  ];
  List<Room> get availableRooms => _availableRooms;

  void createRoom(Room room) {
    _availableRooms.add(room);
    notifyListeners();
  }

  void openRoom(Room room) {
    _currentRoom = room;
    notifyListeners();
  }

  void leaveRoom() {
    _currentRoom = null;
    notifyListeners();
  }
}
