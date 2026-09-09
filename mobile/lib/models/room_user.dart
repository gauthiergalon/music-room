class RoomUser {
  final String id;
  final String username;
  final String deviceName;

  RoomUser({required this.id, required this.username, this.deviceName = 'Unknown device'});

  String get displayName => deviceName.isEmpty ? username : '$username ($deviceName)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RoomUser &&
          runtimeType == other.runtimeType &&
          id == other.id &&
          deviceName == other.deviceName;

  @override
  int get hashCode => Object.hash(id, deviceName);
}
