import 'dart:collection';

import 'package:mobile/models/track.dart';

class Room {
  int id;
  String owner;
  Track? currentTrack;
  final Queue<Track> queue;
  int status; // 0 = waiting, 1 = playing
  final List<String> listeners;
  bool isPublic;

  Duration positionAtLastSync;
  DateTime updatedAt;

  Room({
    required this.id,
    required this.owner,
    this.currentTrack,
    Queue<Track>? queue,
    this.status = 0,
    List<String>? listeners,
    Duration? positionAtLastSync,
    DateTime? updatedAt,
    this.isPublic = true,
  }) : queue = queue ?? Queue(),
       listeners = listeners ?? [],
       positionAtLastSync = positionAtLastSync ?? Duration.zero,
       updatedAt = updatedAt ?? DateTime.now();

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
