class RoomInvitation {
  final String id;
  final String roomId;
  final String inviterId;
  final bool isPending;
  final String inviterUsername;
  final String? roomName;

  const RoomInvitation({
    required this.id,
    required this.roomId,
    required this.inviterId,
    required this.isPending,
    required this.inviterUsername,
    this.roomName,
  });

  factory RoomInvitation.fromJson(Map<String, dynamic> json) {
    return RoomInvitation(
      id: json['id']?.toString() ?? '',
      roomId: json['room_id']?.toString() ?? '',
      inviterId: json['inviter_id']?.toString() ?? '',
      isPending: json['is_pending'] == true,
      inviterUsername: json['inviter_username']?.toString() ?? 'Unknown user',
      roomName: json['room_name']?.toString(),
    );
  }
}
