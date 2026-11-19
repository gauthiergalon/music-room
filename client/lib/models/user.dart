class User {
  final String id;
  final String username;
  final String? email;
  final bool? emailConfirmed;
  final bool googleLinked;
  final bool hasPassword;
  final List<String>? favoriteGenres;
  final String privacyLevel;
  final bool isSubscribed;
  final DateTime? endSubscriptionDate;

  const User({
    required this.id,
    required this.username,
    required this.email,
    this.emailConfirmed,
    this.googleLinked = false,
    this.hasPassword = true,
    this.favoriteGenres,
    this.privacyLevel = 'Friends',
    this.isSubscribed = false,
    this.endSubscriptionDate,
  });

  /// Parses the `UserResponse` returned by `/users/me` and the profile
  /// update endpoints.
  factory User.fromJson(Map<String, dynamic> json) {
    final endDate = json['end_subscription_date'];
    return User(
      id: json['id'].toString(),
      username: json['username'].toString(),
      email: json['email']?.toString(),
      emailConfirmed: json['email_confirmed'] == true,
      googleLinked: json['google_linked'] == true,
      hasPassword: json['has_password'] != false,
      favoriteGenres: parseFavoriteGenres(json['favorite_genres']),
      privacyLevel: json['privacy_level']?.toString() ?? 'Friends',
      isSubscribed: json['is_subscribed'] == true,
      endSubscriptionDate: endDate is String
          ? DateTime.tryParse(endDate)
          : null,
    );
  }

  User copyWith({
    String? username,
    List<String>? favoriteGenres,
    String? privacyLevel,
  }) {
    return User(
      id: id,
      username: username ?? this.username,
      email: email,
      emailConfirmed: emailConfirmed,
      googleLinked: googleLinked,
      hasPassword: hasPassword,
      favoriteGenres: favoriteGenres ?? this.favoriteGenres,
      privacyLevel: privacyLevel ?? this.privacyLevel,
      isSubscribed: isSubscribed,
      endSubscriptionDate: endSubscriptionDate,
    );
  }
}

/// Public view of another user (`GET /users/{id}`).
class PublicProfile {
  final String id;
  final String username;

  /// Null when the profile's privacy level hides the genres from the viewer.
  final List<String>? favoriteGenres;
  final String privacyLevel;

  const PublicProfile({
    required this.id,
    required this.username,
    required this.favoriteGenres,
    required this.privacyLevel,
  });

  factory PublicProfile.fromJson(
    Map<String, dynamic> json, {
    String? fallbackUsername,
  }) {
    final genres = json['favorite_genres'];
    return PublicProfile(
      id: json['id'].toString(),
      username: json['username']?.toString() ?? fallbackUsername ?? 'Unknown',
      favoriteGenres: genres is List ? parseFavoriteGenres(genres) : null,
      privacyLevel: json['privacy_level']?.toString() ?? 'Friends',
    );
  }
}

List<String> parseFavoriteGenres(dynamic raw) {
  if (raw is! List) return const [];
  return raw
      .map((item) => item.toString())
      .where((item) => item.isNotEmpty)
      .toList();
}
