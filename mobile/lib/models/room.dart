class Room {
  final String owner;
  final String currentTrack;
  final int status; // 0 = waiting, 1 = playing
  final int people;

  Room({
    required this.owner,
    required this.currentTrack,
    required this.status,
    required this.people,
  });
}
