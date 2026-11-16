import 'queue_item.dart';
import 'room_user.dart';
import 'track.dart';

class Room {
  final String id;
  String owner;
  String name;
  Track? currentTrack;
  List<QueueItem> queue;
  bool isPlaying;
  List<RoomUser> listeners;
  bool isPublic;
  bool isLicensed;
  String? delegateUserId;
  String? delegateConnectionId;

  Room({
    required this.id,
    required this.owner,
    required this.name,
    this.currentTrack,
    List<QueueItem>? queue,
    this.isPlaying = false,
    List<RoomUser>? listeners,
    this.isPublic = true,
    this.isLicensed = false,
    this.delegateUserId,
    this.delegateConnectionId,
  }) : queue = queue ?? const [],
       listeners = listeners ?? const [];

  factory Room.fromJson(Map<String, dynamic> json) {
    return Room(
      id: json['id'].toString(),
      owner: json['owner_id'].toString(),
      name: json['name']?.toString() ?? 'Unnamed Room',
      isPublic: json['is_public'] != false,
      isLicensed: json['is_licensed'] == true,
      isPlaying: json['is_playing'] == true,
    );
  }
}
