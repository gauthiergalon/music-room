import 'package:mobile/models/track.dart';
import 'package:mobile/models/track_cache.dart';

class QueueItem {
  final String id;
  final String roomId;
  final int trackId;
  final double position;
  final Track track;

  QueueItem({
    required this.id,
    required this.roomId,
    required this.trackId,
    required this.position,
  }) : track = getCachedTrack(trackId);

  factory QueueItem.fromJson(Map<String, dynamic> json) {
    return QueueItem(
      id: json['id'],
      roomId: json['room_id'],
      trackId: json['track_id'],
      position: json['position'].toDouble(),
    );
  }
}
