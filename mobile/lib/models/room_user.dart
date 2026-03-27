class RoomUser {
  final String id;
  final String username;

  RoomUser({required this.id, required this.username});

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RoomUser && runtimeType == other.runtimeType && id == other.id;

  @override
  int get hashCode => id.hashCode;
}
