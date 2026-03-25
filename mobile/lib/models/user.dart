class User {
  final String id;
  final String username;
  final String? email;
  final bool? emailConfirmed;
  final String? googleId;

  User({
    required this.id,
    required this.username,
    required this.email,
    this.emailConfirmed,
    this.googleId,
  });

  User copyWith({
    String? id,
    String? username,
    String? email,
    bool? emailConfirmed,
    String? googleId,
  }) {
    return User(
      id: id ?? this.id,
      username: username ?? this.username,
      email: email ?? this.email,
      emailConfirmed: emailConfirmed ?? this.emailConfirmed,
      googleId: googleId ?? this.googleId,
    );
  }
}
