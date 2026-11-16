import 'track.dart';

class QueueItem {
  final String id;
  final double position;
  final Track track;

  const QueueItem({
    required this.id,
    required this.position,
    required this.track,
  });

  int get trackId => track.id;

  /// Parses one entry of the `queue` array of a `RoomState` message.
  factory QueueItem.fromJson(Map<String, dynamic> json, {double? fallback}) {
    return QueueItem(
      id: json['id'].toString(),
      position: (json['position'] as num?)?.toDouble() ?? fallback ?? 0,
      track: Track.fromJson(Map<String, dynamic>.from(json['track'] as Map)),
    );
  }
}
