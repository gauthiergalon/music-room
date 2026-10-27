class RoomUser {
  final String id;
  final String username;
  final String deviceName;

  final String connectionId;

  RoomUser({required this.id, required this.username, required this.connectionId, this.deviceName = 'Unknown device'});

  String get displayName => deviceName.isEmpty ? username : '$username ($deviceName)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RoomUser &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          connectionId == other.connectionId;

  @override
  int get hashCode => Object.hash(id, connectionId);
}
