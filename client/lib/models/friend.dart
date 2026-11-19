class Friend {
  /// Id of the other user of the friendship.
  final String friendId;
  final String? username;
  final bool isPending;

  /// True when the current user sent the request.
  final bool isSender;

  const Friend({
    required this.friendId,
    required this.isPending,
    required this.isSender,
    this.username,
  });

  factory Friend.fromJson(Map<String, dynamic> json) {
    final friendId = json['friend_id']?.toString() ?? '';
    return Friend(
      friendId: friendId,
      username: json['username']?.toString(),
      isPending: json['is_pending'] == true,
      isSender: json['sender_id']?.toString() != friendId,
    );
  }
}
