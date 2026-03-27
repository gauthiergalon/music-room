import 'package:mobile/models/track.dart';
import 'package:mobile/models/mock_tracks.dart';
import 'package:mobile/models/queue_item.dart';
import 'package:mobile/models/room_user.dart';

class Room {
  String id;
  String owner;
  String name;
  Track? currentTrack;
  List<QueueItem> queue;
  int status; // 0 = waiting, 1 = playing
  List<RoomUser> listeners;
  bool isPublic;

  Duration positionAtLastSync;
  DateTime updatedAt;

  Room({
    required this.id,
    required this.owner,
    required this.name,
    this.currentTrack,
    List<QueueItem>? queue,
    this.status = 0,
    List<RoomUser>? listeners,
    Duration? positionAtLastSync,
    DateTime? updatedAt,
    this.isPublic = true,
  }) : queue = queue ?? [],
       listeners = listeners ?? [],
       positionAtLastSync = positionAtLastSync ?? Duration.zero,
       updatedAt = updatedAt ?? DateTime.now();

  factory Room.fromJson(Map<String, dynamic> json) {
    Track? cTrack;
    if (json['current_track'] != null) {
      cTrack = getMockTrack(json['current_track']);
    }

    return Room(
      id: json['id'],
      owner: json['owner_id'],
      name: json['name'] ?? 'Unnamed Room',
      currentTrack: cTrack,
      isPublic: json['is_public'] ?? true,
      status: (json['is_playing'] == true) ? 1 : 0,
      positionAtLastSync: Duration(milliseconds: (json['current_position']?.toDouble() ?? 0.0).toInt()),
    );
  }

  Duration get currentPosition {
    if (currentTrack == null) return Duration.zero;
    if (status != 1) return positionAtLastSync;

    final now = DateTime.now();
    final elapsed = now.difference(updatedAt);
    final calculatedPos = positionAtLastSync + elapsed;

    if (calculatedPos > currentTrack!.duration) {
      return currentTrack!.duration;
    }
    return calculatedPos;
  }

  int get people => listeners.length;
}
