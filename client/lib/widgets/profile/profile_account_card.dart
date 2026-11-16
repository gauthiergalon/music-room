import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../controllers/auth_controller.dart';
import '../../core/utils/ui_utils.dart';
import '../../models/user.dart';
import '../edit_dialog.dart';
import '../profile_info_tile.dart';
import 'profile_action.dart';
import 'profile_dialogs.dart';

/// Identity and preferences of the logged-in user.
class ProfileAccountCard extends StatefulWidget {
  const ProfileAccountCard({super.key, required this.user});

  final User user;

  @override
  State<ProfileAccountCard> createState() => _ProfileAccountCardState();
}

class _ProfileAccountCardState extends State<ProfileAccountCard> {
  bool _isUpdatingGenres = false;
  bool _isUpdatingPrivacy = false;

  AuthController get _auth => context.read<AuthController>();

  void _editUsername() {
    showEditDialog(
      context,
      title: 'Username',
      currentValue: widget.user.username,
      onSave: (newValue) => runProfileAction(
        context,
        () => _auth.updateUsername(newValue),
        successMessage: 'Username updated successfully',
      ),
    );
  }

  void _editEmail() {
    showEditDialog(
      context,
      title: 'Email Address',
      currentValue: widget.user.email ?? '',
      isEmail: true,
      onSave: (newValue) => runProfileAction(
        context,
        () => _auth.updateEmail(newValue),
        successMessage:
            'Confirmation email sent to $newValue. '
            'The change applies once confirmed.',
      ),
    );
  }

  void _editPassword() {
    if (!widget.user.hasPassword) {
      UiUtils.showSnackBar(
        context,
        'This account has no password yet. Use "Forgot your password?" on '
        'the login screen to set one.',
      );
      return;
    }

    showPasswordDialog(
      context,
      onSave: (passwords) => runProfileAction(
        context,
        () => _auth.updatePassword(passwords.current, passwords.next),
        successMessage: 'Password updated successfully',
      ),
    );
  }

  void _editGenres() {
    showFavoriteGenresDialog(
      context,
      current: widget.user.favoriteGenres ?? const [],
      onSave: (genres) async {
        setState(() => _isUpdatingGenres = true);
        try {
          return await runProfileAction(
            context,
            () => _auth.updateFavoriteGenres(genres),
            successMessage: 'Music tastes updated successfully',
          );
        } finally {
          if (mounted) setState(() => _isUpdatingGenres = false);
        }
      },
    );
  }

  Future<void> _editPrivacy() async {
    final selected = await showPrivacyDialog(context, widget.user.privacyLevel);
    if (selected == null || !mounted) return;

    setState(() => _isUpdatingPrivacy = true);
    try {
      await runProfileAction(
        context,
        () => _auth.updatePrivacyLevel(selected),
        successMessage: 'Privacy updated successfully',
      );
    } finally {
      if (mounted) setState(() => _isUpdatingPrivacy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final user = widget.user;
    final genres = user.favoriteGenres ?? const [];

    return Card(
      child: Column(
        children: [
          ProfileInfoTile(
            icon: Icons.person_outline,
            title: 'Username',
            subtitle: user.username,
            onTap: _editUsername,
          ),
          ProfileInfoTile(
            icon: Icons.email_outlined,
            title: 'Email Address',
            subtitle: user.email ?? 'No email linked',
            onTap: _editEmail,
          ),
          ProfileInfoTile(
            icon: Icons.lock_outline,
            title: 'Password',
            subtitle: user.hasPassword ? '********' : 'Not set',
            onTap: _editPassword,
          ),
          ListTile(
            leading: const Icon(Icons.library_music_outlined),
            title: const Text('Music Tastes'),
            subtitle: genres.isEmpty
                ? const Text('Tap to add your favorite genres')
                : Wrap(
                    spacing: 6,
                    runSpacing: 4,
                    children: [
                      for (final genre in genres)
                        Chip(
                          label: Text(
                            genre,
                            style: const TextStyle(fontSize: 12),
                          ),
                          backgroundColor: Theme.of(
                            context,
                          ).colorScheme.surfaceContainerHighest,
                          side: BorderSide.none,
                        ),
                    ],
                  ),
            trailing: _trailing(_isUpdatingGenres),
            onTap: _isUpdatingGenres ? null : _editGenres,
          ),
          ListTile(
            leading: const Icon(Icons.visibility_outlined),
            title: const Text('Profile Privacy'),
            subtitle: Text(_privacyLabel(user.privacyLevel)),
            trailing: _trailing(_isUpdatingPrivacy),
            onTap: _isUpdatingPrivacy ? null : _editPrivacy,
          ),
        ],
      ),
    );
  }

  Widget _trailing(bool isUpdating) {
    return isUpdating
        ? const SizedBox(
            width: 18,
            height: 18,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : const Icon(Icons.chevron_right);
  }

  static String _privacyLabel(String value) {
    switch (value.toLowerCase()) {
      case 'public':
        return 'Public';
      case 'private':
        return 'Private';
      case 'friends':
      default:
        return 'Friends only';
    }
  }
}
