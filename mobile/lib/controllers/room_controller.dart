import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/models/queue_item.dart';
import '../models/room.dart';
import '../core/network/api_client.dart';
import '../core/network/ws_factory.dart';
import '../models/room_user.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

class RoomController extends ChangeNotifier {
  Room? _currentRoom;
  Room? get currentRoom => _currentRoom;

  List<Room> _availableRooms = [];
  List<Room> get availableRooms => _availableRooms;

  WebSocketChannel? _wsChannel;

  Future<void> refreshRooms() async {
    try {
      final response = await ApiClient.get('/rooms');
      if (response is List) {
        _availableRooms = response.map((data) => Room.fromJson(data)).toList();
        notifyListeners();
      }
    } catch (e) {
      debugPrint('Failed to refresh rooms: $e');
      rethrow;
    }
  }

  Future<Room> createRoom() async {
    try {
      final response = await ApiClient.post('/rooms');
      final newRoom = Room.fromJson(response);
      _availableRooms.add(newRoom);
      notifyListeners();
      await openRoom(newRoom);
      return newRoom;
    } catch (e) {
      debugPrint('Failed to create room: $e');
      rethrow;
    }
  }

  Future<void> openRoom(Room room) async {
    _currentRoom = room;
    await refreshQueue(room.id);
    _connectWebSocket(room.id);
    notifyListeners();
  }

  void leaveRoom() {
    _currentRoom = null;
    _wsChannel?.sink.close();
    _wsChannel = null;
    notifyListeners();

    Future.delayed(const Duration(milliseconds: 100), () {
      refreshRooms();
    });
  }

  void _connectWebSocket(String roomId) async {
    final token = await ApiClient.getToken();
    if (token == null) return;

    final baseUrl =
        '${ApiClient.baseUrl.replaceFirst('http', 'ws')}/rooms/$roomId/ws';
    _wsChannel = createWsChannel(baseUrl, token);

    try {
      _wsChannel!.stream.listen(
        (message) {
          debugPrint('WS Message: $message');
          final data = jsonDecode(message);
          _handleWsEvent(data);
        },
        onError: (err) => debugPrint('WS Error: $err'),
        onDone: () => debugPrint('WS Closed'),
      );
    } catch (e) {
      debugPrint('Error listening to WS: $e');
    }
  }

  void _handleWsEvent(Map<String, dynamic> data) {
    if (_currentRoom == null) return;
    final type = data['type'];
    final payload = data['payload'] ?? {};
    if (type == 'SyncUsers') {
      final users = payload['users'] as List?;
      if (users != null) {
        _currentRoom!.listeners.clear();
        for (var u in users) {
          final id = u['user_id'] as String?;
          final name = u['username'] as String?;
          if (id != null && name != null) {
            _currentRoom!.listeners.add(RoomUser(id: id, username: name));
          }
        }
      }
    } else if (type == 'UserJoin') {
      final id = payload['user_id'] as String?;
      final name = payload['username'] as String?;
      if (id != null && name != null) {
        final existing = _currentRoom!.listeners.indexWhere((u) => u.id == id);
        if (existing == -1) {
          _currentRoom!.listeners.add(RoomUser(id: id, username: name));
        }
      }
    } else if (type == 'UserLeave') {
      final id = payload['user_id'] as String?;
      if (id != null) {
        _currentRoom!.listeners.removeWhere((u) => u.id == id);
      }
    } else if (type == 'RoomClosed') {
      leaveRoom();
      return;
    }

    // React to events (add simple logs or state updates)
    if (type == 'QueueAdd' || type == 'QueueRemove' || type == 'QueueReorder') {
      refreshQueue(_currentRoom!.id);
    }
    notifyListeners();
  }

  Future<void> refreshQueue(String roomId) async {
    try {
      final response = await ApiClient.get('/rooms/$roomId/queue');
      if (response is List) {
        if (_currentRoom != null && _currentRoom!.id == roomId) {
          _currentRoom!.queue = response
              .map((data) => QueueItem.fromJson(data))
              .toList();
          notifyListeners();
        }
      }
    } catch (e) {
      debugPrint('Failed to refresh queue: $e');
    }
  }

  // Queue management
  Future<void> addTrack(Room room, Track track) async {
    try {
      await ApiClient.post(
        '/rooms/${room.id}/queue',
        body: {'track_id': track.id},
      );
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to add track: $e');
    }
  }

  Future<void> removeQueueItem(Room room, QueueItem item) async {
    try {
      await ApiClient.delete('/rooms/${room.id}/queue', body: {'id': item.id});
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to remove track: $e');
    }
  }

  Future<void> moveQueueItem(
    Room room,
    QueueItem item,
    double newPosition,
  ) async {
    try {
      await ApiClient.patch(
        '/rooms/${room.id}/queue',
        body: {'id': item.id, 'new_position': newPosition},
      );
      await refreshQueue(room.id);
    } catch (e) {
      debugPrint('Failed to reorder track: $e');
    }
  }

  // Music control
  void togglePlay(Room room) {
    // Should call WS event or endpoint
  }

  void seekTo(Room room, Duration position) {
    // Should call WS event or endpoint
  }

  void skipNext() {
    // Should call WS event or endpoint
  }

  void skipPrev() {
    // Should call WS event or endpoint
  }

  Future<void> togglePrivacy(Room room) async {
    try {
      if (!room.isPublic) {
        await ApiClient.post('/rooms/${room.id}/publish');
      } else {
        await ApiClient.post('/rooms/${room.id}/privatize');
      }
      room.isPublic = !room.isPublic;
      notifyListeners();

      refreshRooms(); // Update available list
    } catch (e) {
      debugPrint('Failed to toggle privacy: $e');
      notifyListeners(); // Keep UI in sync if the switch failed
      rethrow;
    }
  }

  void kickListener(Room room, RoomUser listener) {}
  Future<void> promoteToOwner(Room room, RoomUser listener) async {
    try {
      await ApiClient.post(
        '/rooms/${room.id}/transfer-ownership',
        body: {'new_owner_id': listener.id},
      );
    } catch (e) {
      debugPrint('Failed to transfer ownership: $e');
    }
  }
}
