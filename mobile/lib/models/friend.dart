class Friend {
  final String userId1;
  final String userId2;
  final String senderId;
  final bool isPending;

  // App-specific resolved fields
  String friendId;
  bool isSender;
  String? username;

  Friend({
    required this.userId1,
    required this.userId2,
    required this.senderId,
    required this.isPending,
    required this.friendId,
    required this.isSender,
    this.username,
  });

  factory Friend.fromJson(Map<String, dynamic> json, String? currentUserId) {
    final String u1 = json['user_id_1']?.toString() ?? '';
    final String u2 = json['user_id_2']?.toString() ?? '';
    final String resolvedFriendId = (u1 == currentUserId) ? u2 : u1;

    // Some backend versions might use 'status' instead of 'is_pending'
    final isPending = json['is_pending'] == true || json['status'] == 'pending';

    return Friend(
      userId1: u1,
      userId2: u2,
      senderId: json['sender_id']?.toString() ?? '',
      isPending: isPending,
      friendId: resolvedFriendId,
      isSender: json['sender_id']?.toString() == currentUserId,
      username: json['username'], // Pre-filled if already available
    );
  }
}
