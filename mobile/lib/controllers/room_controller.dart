import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:mobile/models/track.dart';
import '../models/room.dart';

class RoomController extends ChangeNotifier {
  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;

  final List<Room> _availableRooms = [
    Room(
      id: 1,
      owner: 'Alice',
      currentTrack: Track(
        id: 1,
        title: 'Song A',
        artist: 'Artist X',
        imageUrl: 'https://picsum.photos/400',
        duration: const Duration(minutes: 3, seconds: 30),
      ),
      status: 0,
      listeners: ['Alice', 'Anna', 'Armand'],
    ),
    Room(
      id: 2,
      owner: 'Bob',
      currentTrack: Track(
        id: 2,
        title: 'Song B',
        artist: 'Artist Y',
        imageUrl: 'https://picsum.photos/400',
        duration: const Duration(minutes: 3, seconds: 30),
      ),
      status: 0,
      listeners: ['Bob', 'Bella'],
    ),
    Room(
      id: 3,
      owner: 'Charlie',
      currentTrack: Track(
        id: 3,
        title: 'Song C',
        artist: 'Artist Z',
        imageUrl: 'https://picsum.photos/400',
        duration: const Duration(minutes: 3, seconds: 30),
      ),
      status: 0,
      queue: Queue.of([
        Track(
          id: 4,
          title: 'Song D',
          artist: 'Artist W',
          imageUrl: 'https://picsum.photos/400',
          duration: const Duration(minutes: 3, seconds: 30),
        ),
        Track(
          id: 5,
          title: 'Song E',
          artist: 'Artist V',
          imageUrl: 'https://picsum.photos/400',
          duration: const Duration(minutes: 3, seconds: 30),
        ),
      ]),
      listeners: ['Charlie'],
    ),
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

  Future<void> refreshRooms() async {
    await Future.delayed(const Duration(milliseconds: 800));
    _availableRooms.shuffle();
    notifyListeners();
  }

  // Queue management
  void addTrack(Room room, Track track) {
    room.queue.add(track);
    if (room.currentTrack == null) {
      skipNext();
    }
    notifyListeners();
  }

  void removeTrack(Room room, int index) {
    final list = room.queue.toList();
    if (index < 0 || index >= list.length) return;
    list.removeAt(index);
    room.queue.clear();
    room.queue.addAll(list);
    notifyListeners();
  }

  void playTrack(Room room, int index) {
    final list = room.queue.toList();
    if (index < 0 || index >= list.length) return;
    room.currentTrack = list[index];
    room.status = 1;
    room.positionAtLastSync = Duration.zero;
    room.updatedAt = DateTime.now();
    notifyListeners();
  }

  void moveTrack(Room room, int from, int to) {
    final list = room.queue.toList();
    if (from < 0 || from >= list.length || to < 0 || to >= list.length) return;
    final item = list.removeAt(from);
    list.insert(to, item);
    room.queue.clear();
    room.queue.addAll(list);
    notifyListeners();
  }

  // Listener management
  void togglePrivacy(Room room) {
    room.isPublic = !room.isPublic;
    notifyListeners();
  }

  void kickListener(Room room, String listener) {
    if (room.listeners.contains(listener) && listener != room.owner) {
      room.listeners.remove(listener);
      notifyListeners();
    }
  }

  void promoteToOwner(Room room, String listener) {
    if (room.listeners.contains(listener)) {
      final previous = room.owner;
      room.owner = listener;
      if (!room.listeners.contains(previous)) {
        room.listeners.insert(0, previous);
      }
      notifyListeners();
    }
  }

  // Music control
  void togglePlay(Room room) {
    final now = DateTime.now();
    if (room.status == 1) {
      // Pause
      room.positionAtLastSync = room.currentPosition;
      room.status = 0;
    } else {
      // Play
      if (room.currentPosition >=
          (room.currentTrack?.duration ?? Duration.zero)) {
        room.positionAtLastSync = Duration.zero;
      } else {
        room.positionAtLastSync = room.currentPosition;
      }
      room.status = 1;
      room.updatedAt = now;
    }
    notifyListeners();
  }

  void seekTo(Room room, Duration position) {
    room.positionAtLastSync = position;
    room.updatedAt = DateTime.now();
    notifyListeners();
  }

  void skipNext() {
    final room = _currentRoom;
    if (room == null) return;
    if (room.queue.isNotEmpty) {
      final next = room.queue.removeFirst();
      room.currentTrack = next;
      room.status = 1;
      room.positionAtLastSync = Duration.zero;
      room.updatedAt = DateTime.now();
    } else {
      room.currentTrack = null;
      room.status = 0;
      room.positionAtLastSync = Duration.zero;
      room.updatedAt = DateTime.now();
    }
    notifyListeners();
  }

  void skipPrev() {
    final room = _currentRoom;
    if (room == null) return;
    if (room.currentTrack != null) {
      room.status = 1;
      room.positionAtLastSync = Duration.zero;
      room.updatedAt = DateTime.now();
      notifyListeners();
    }
  }
}
