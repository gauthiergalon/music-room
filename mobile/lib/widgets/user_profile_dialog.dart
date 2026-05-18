import 'package:flutter/material.dart';
import '../core/network/api_client.dart';
import '../models/user.dart';

Future<void> showUserProfileDialog(
  BuildContext context,
  String userId, {
  String? initialUsername,
}) async {
  final scaffoldMessenger = ScaffoldMessenger.of(context);

  try {
    final data = await ApiClient.getUser(userId);
    final user = User(
      id: data['id'].toString(),
      username: data['username'] ?? initialUsername ?? 'Unknown',
      email: data['email'],
      emailConfirmed: data['email_confirmed'],
      googleId: data['google_id'],
      favoriteGenres: _parseFavoriteGenres(data['favorite_genres']),
      privacyLevel: data['privacy_level']?.toString() ?? 'Friends',
    );

    if (!context.mounted) return;
    _buildDialog(context, user);
  } catch (e) {
    scaffoldMessenger.showSnackBar(
      SnackBar(
        content: Text('Failed to load profile: $e'),
        backgroundColor: Colors.red,
        behavior: SnackBarBehavior.floating,
      ),
    );
  }
}

void _buildDialog(BuildContext context, User user) {
  showDialog<void>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Row(
        children: [
          CircleAvatar(
            radius: 24,
            backgroundColor: Theme.of(ctx).colorScheme.primary,
            child: Text(
              user.username.isNotEmpty ? user.username[0].toUpperCase() : '?',
              style: const TextStyle(
                fontSize: 20,
                fontWeight: FontWeight.bold,
                color: Colors.white,
              ),
            ),
          ),
          const SizedBox(width: 16),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(user.username, style: const TextStyle(fontSize: 18)),
                if (user.email != null)
                  Text(
                    user.email!,
                    style: TextStyle(
                      fontSize: 14,
                      color: Theme.of(ctx).colorScheme.onSurfaceVariant,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (user.emailConfirmed != null)
            _buildInfoRow(
              ctx,
              'Email verified',
              user.emailConfirmed! ? Icons.check_circle : Icons.cancel,
              user.emailConfirmed! ? Colors.green : Colors.red,
            ),
          _buildMusicTastesSection(ctx, user),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(),
          child: const Text('Close'),
        ),
      ],
    ),
  );
}

Widget _buildMusicTastesSection(BuildContext ctx, User user) {
  final isPrivate = user.privacyLevel.toLowerCase() == 'private';
  final genres = user.favoriteGenres ?? [];
  final hasGenres = genres.isNotEmpty;

  // Si private ou pas d'accès (favoriteGenres == null), affiche "This profile is private"
  if (isPrivate || user.favoriteGenres == null) {
    return Text(
      'This profile is private',
      style: TextStyle(
        fontSize: 14,
        fontStyle: FontStyle.italic,
        color: Theme.of(ctx).colorScheme.onSurfaceVariant,
      ),
    );
  }

  // On a accès (amis ou public, genres définis ou non)
  return Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      const Text('Music Tastes', style: TextStyle(fontWeight: FontWeight.bold)),
      const SizedBox(height: 8),
      if (!hasGenres)
        const Text('None', style: TextStyle(fontSize: 14))
      else
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: genres.map((genre) {
            return Chip(
              label: Text(genre),
              visualDensity: VisualDensity.compact,
            );
          }).toList(),
        ),
    ],
  );
}

List<String>? _parseFavoriteGenres(dynamic rawFavoriteGenres) {
  if (rawFavoriteGenres is! List) {
    return null;
  }
  final favoriteGenres = rawFavoriteGenres
      .map((item) => item.toString())
      .where((item) => item.isNotEmpty)
      .toList();
  return favoriteGenres.isEmpty ? [] : favoriteGenres;
}

Widget _buildInfoRow(
  BuildContext context,
  String label,
  IconData icon,
  Color iconColor, {
  String? subtitle,
}) {
  return Row(
    children: [
      Icon(icon, size: 20, color: iconColor),
      const SizedBox(width: 8),
      Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: const TextStyle(fontWeight: FontWeight.bold)),
          if (subtitle != null)
            Text(
              subtitle,
              style: TextStyle(
                fontSize: 12,
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
        ],
      ),
    ],
  );
}
