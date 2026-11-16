import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../controllers/auth_controller.dart';
import '../../core/exceptions/api_exception.dart';
import '../../core/utils/ui_utils.dart';
import '../../models/user.dart';
import '../../screens/friends_page.dart';
import '../google/google_web_sign_in.dart';
import 'profile_action.dart';

/// Friends, email verification, Google link and subscription.
class ProfileSettingsCard extends StatelessWidget {
  const ProfileSettingsCard({super.key, required this.user});

  final User user;

  Future<void> _sendVerificationEmail(BuildContext context) async {
    try {
      await context.read<AuthController>().sendEmailConfirmation();
      if (context.mounted) {
        UiUtils.showSuccess(
          context,
          'Verification email sent! Check your inbox.',
        );
      }
    } on ApiException catch (e) {
      if (!context.mounted) return;
      final message = switch (e.statusCode) {
        429 =>
          'An email was already sent recently, please check your inbox or '
              'try again later',
        409 => 'Email is already verified',
        _ => e.message,
      };
      UiUtils.showError(context, message);
    } catch (e) {
      if (context.mounted) UiUtils.showError(context, e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    final emailConfirmed = user.emailConfirmed == true;

    return Card(
      child: Column(
        children: [
          ListTile(
            leading: const Icon(Icons.people_outline),
            title: const Text('Manage Friends'),
            subtitle: const Text('View and add friends'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () {
              Navigator.push(
                context,
                MaterialPageRoute(builder: (_) => const FriendsPage()),
              );
            },
          ),
          ListTile(
            leading: const Icon(Icons.mark_email_read_outlined),
            title: const Text('Email Verification'),
            subtitle: Text(
              emailConfirmed
                  ? 'Your email is verified'
                  : 'Tap to send a verification email',
            ),
            trailing: emailConfirmed
                ? const Icon(Icons.check_circle, color: Colors.green)
                : const Icon(Icons.chevron_right),
            onTap: emailConfirmed
                ? null
                : () => _sendVerificationEmail(context),
          ),
          ListTile(
            leading: const Icon(Icons.account_circle_outlined),
            title: const Text('Google Account'),
            subtitle: user.googleLinked
                ? const Text('Linked to Google')
                : kIsWeb
                // On the web, Google only signs in through its own button.
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text('Link it to sign in with Google'),
                      const SizedBox(height: 10),
                      GoogleWebSignIn(
                        alignment: AlignmentDirectional.centerStart,
                        compact: true,
                        onIdToken: (idToken) => context
                            .read<AuthController>()
                            .linkGoogleAccount(idToken: idToken),
                        successMessage: 'Google account linked successfully!',
                        fallbackError:
                            'An unexpected error occurred while linking Google.',
                      ),
                      const SizedBox(height: 4),
                    ],
                  )
                : const Text('Tap to link your Google account'),
            trailing: user.googleLinked
                ? const Icon(Icons.check_circle, color: Colors.green)
                : kIsWeb
                ? null
                : const Icon(Icons.link),
            onTap: user.googleLinked || kIsWeb
                ? null
                : () => runProfileAction(
                    context,
                    context.read<AuthController>().linkGoogleAccount,
                    successMessage: 'Google account linked successfully!',
                    fallbackError:
                        'An unexpected error occurred while linking Google.',
                  ),
          ),
          ListTile(
            leading: const Icon(Icons.diamond_outlined),
            title: const Text('Paid Subscription'),
            subtitle: Text(
              user.isSubscribed
                  ? 'Active. Private rooms are unlocked.'
                  : 'Tap to subscribe. Private rooms are premium only.',
            ),
            trailing: user.isSubscribed
                ? const Icon(Icons.check_circle)
                : const Icon(Icons.workspace_premium_outlined),
            onTap: user.isSubscribed
                ? null
                : () => runProfileAction(
                    context,
                    context.read<AuthController>().enableSubscription,
                    successMessage: 'Subscription created successfully!',
                    fallbackError:
                        'An unexpected error occurred while creating '
                        'subscription.',
                  ),
          ),
        ],
      ),
    );
  }
}
