import 'package:flutter/material.dart';
import '../models/user.dart';
import '../services/user_repository.dart';

Future<void> showUserProfileDialog(
  BuildContext context,
  String userId, {
  String? initialUsername,
  UserRepository userRepository = const UserRepository(),
}) async {
  final scaffoldMessenger = ScaffoldMessenger.of(context);

  try {
    final profile = await userRepository.getPublicProfile(
      userId,
      fallbackUsername: initialUsername,
    );

    if (!context.mounted) return;
    _buildDialog(context, profile);
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

void _buildDialog(BuildContext context, PublicProfile user) {
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
            child: Text(user.username, style: const TextStyle(fontSize: 18)),
          ),
        ],
      ),
      content: _buildMusicTastesSection(ctx, user),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(ctx).pop(),
          child: const Text('Close'),
        ),
      ],
    ),
  );
}

Widget _buildMusicTastesSection(BuildContext ctx, PublicProfile user) {
  final genres = user.favoriteGenres;

  // No genres in the answer means the viewer is not allowed to see them.
  if (genres == null || user.privacyLevel.toLowerCase() == 'private') {
    return Text(
      'This profile is private',
      style: TextStyle(
        fontSize: 14,
        fontStyle: FontStyle.italic,
        color: Theme.of(ctx).colorScheme.onSurfaceVariant,
      ),
    );
  }

  return Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      const Text('Music Tastes', style: TextStyle(fontWeight: FontWeight.bold)),
      const SizedBox(height: 8),
      if (genres.isEmpty)
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
