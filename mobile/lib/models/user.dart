class User {
  final String id;
  final String username;
  final String? email;
  final bool? emailConfirmed;
  final String? googleId;
  final List<String>? favoriteGenres;
  final String privacyLevel;

  User({
    required this.id,
    required this.username,
    required this.email,
    this.emailConfirmed,
    this.googleId,
    this.favoriteGenres,
    this.privacyLevel = 'Friends',
  });

  User copyWith({
    String? id,
    String? username,
    String? email,
    bool? emailConfirmed,
    String? googleId,
    List<String>? favoriteGenres,
    String? privacyLevel,
  }) {
    return User(
      id: id ?? this.id,
      username: username ?? this.username,
      email: email ?? this.email,
      emailConfirmed: emailConfirmed ?? this.emailConfirmed,
      googleId: googleId ?? this.googleId,
      favoriteGenres: favoriteGenres ?? this.favoriteGenres,
      privacyLevel: privacyLevel ?? this.privacyLevel,
    );
  }
}
