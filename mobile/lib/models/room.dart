import 'package:mobile/models/queue_item.dart';
import 'package:mobile/models/room_user.dart';
import 'package:mobile/models/track.dart';
import 'package:mobile/models/track_cache.dart';

class Room {
  String id;
  String owner;
  String name;
  Track? currentTrack;
  List<QueueItem> queue;
  int status;
  List<RoomUser> listeners;
  bool isPublic;
  bool isLicensed;

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
    this.isLicensed = false,
  }) : queue = queue ?? [],
       listeners = listeners ?? [],
       positionAtLastSync = positionAtLastSync ?? Duration.zero,
       updatedAt = updatedAt ?? DateTime.now();

  factory Room.fromJson(Map<String, dynamic> json) {
    final Track? cTrack = json['current_track'] != null
        ? getCachedTrack(json['current_track'])
        : null;

    return Room(
      id: json['id'],
      owner: json['owner_id'],
      name: json['name'] ?? 'Unnamed Room',
      currentTrack: cTrack,
      isPublic: json['is_public'] ?? true,
      isLicensed: json['has_license'] ?? json['is_licensed'] ?? false,
      status: (json['is_playing'] == true) ? 1 : 0,
      positionAtLastSync: Duration(
        milliseconds: (json['current_position']?.toDouble() ?? 0.0).toInt(),
      ),
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
