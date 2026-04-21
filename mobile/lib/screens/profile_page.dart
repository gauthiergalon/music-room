import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../core/theme.dart';
import '../widgets/edit_dialog.dart';
import '../widgets/profile_info_tile.dart';
import '../controllers/auth_controller.dart';
import '../core/utils/ui_utils.dart';
import '../core/exceptions/api_exception.dart';
import 'friends_page.dart';

class ProfilePage extends StatefulWidget {
  const ProfilePage({super.key});

  @override
  State<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  @override
  Widget build(BuildContext context) {
    final authController = context.watch<AuthController>();
    final user = authController.user;

    if (user == null) {
      if (authController.isLoadingUser) {
        return const Scaffold(body: Center(child: CircularProgressIndicator()));
      }
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.error_outline,
                size: 48,
                color: Colors.redAccent,
              ),
              const SizedBox(height: 16),
              const Text('An error occurred while loading data.'),
              const SizedBox(height: 16),
              ElevatedButton.icon(
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
                onPressed: () => authController.fetchUserInfo(),
              ),
            ],
          ),
        ),
      );
    }

    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: ListView(
          padding: const EdgeInsets.all(AppTheme.spacingMd),
          children: [
            Card(
              child: Column(
                children: [
                  ProfileInfoTile(
                    icon: Icons.person_outline,
                    title: 'Username',
                    subtitle: user.username,
                    onTap: () {
                      showEditDialog(
                        context,
                        title: 'Username',
                        currentValue: user.username,
                        onSave: (newValue) async {
                          final currentContext = context;
                          try {
                            await context.read<AuthController>().updateUsername(
                              newValue,
                            );
                            if (currentContext.mounted) {
                              UiUtils.showSuccess(
                                currentContext,
                                'Username updated successfully',
                              );
                            }
                          } on ApiException catch (e) {
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.message);
                            }
                          } catch (e) {
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.toString());
                            }
                          }
                        },
                      );
                    },
                  ),
                  ProfileInfoTile(
                    icon: Icons.email_outlined,
                    title: 'Email Address',
                    subtitle: user.email ?? 'No email linked',
                    onTap: () {
                      showEditDialog(
                        context,
                        title: 'Email Address',
                        currentValue: user.email ?? '',
                        isEmail: true,
                        onSave: (newValue) async {
                          final currentContext = context;
                          try {
                            await context.read<AuthController>().updateEmail(
                              newValue,
                            );
                            if (currentContext.mounted) {
                              UiUtils.showSuccess(
                                currentContext,
                                'Email updated successfully',
                              );
                            }
                          } on ApiException catch (e) {
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.message);
                            }
                          } catch (e) {
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.toString());
                            }
                          }
                        },
                      );
                    },
                  ),
                  ProfileInfoTile(
                    icon: Icons.lock_outline,
                    title: 'Password',
                    subtitle: '********',
                    onTap: () {
                      _showPasswordDialog(context);
                    },
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            Card(
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
                    leading: Icon(Icons.mark_email_read_outlined),
                    title: const Text('Email Verification'),
                    subtitle: Text(
                      user.emailConfirmed == true
                          ? 'Your email is verified'
                          : 'Tap to send a verification email',
                    ),
                    trailing: user.emailConfirmed == true
                        ? const Icon(Icons.check_circle, color: Colors.green)
                        : const Icon(Icons.chevron_right),
                    onTap: user.emailConfirmed == true
                        ? null
                        : () async {
                            final currentContext = context;
                            try {
                              await context
                                  .read<AuthController>()
                                  .sendEmailConfirmation();
                              if (currentContext.mounted) {
                                UiUtils.showSuccess(
                                  currentContext,
                                  'Verification email sent! Check your inbox.',
                                );
                              }
                            } on ApiException catch (e) {
                              if (currentContext.mounted) {
                                UiUtils.showError(currentContext, e.message);
                              }
                            } catch (e) {
                              if (currentContext.mounted) {
                                UiUtils.showError(currentContext, e.toString());
                              }
                            }
                          },
                  ),
                  ListTile(
                    leading: const Icon(Icons.account_circle_outlined),
                    title: const Text('Google Account'),
                    subtitle: Text(
                      user.googleId != null
                          ? 'Linked to Google'
                          : 'Tap to link your Google account',
                    ),
                    trailing: user.googleId != null
                        ? const Icon(Icons.check_circle)
                        : const Icon(Icons.link),
                    onTap: user.googleId != null
                        ? null
                        : () {
                            UiUtils.showError(
                              context,
                              'Google linking is not implemented yet',
                            );
                          },
                  ),
                ],
              ),
            ),
            const SizedBox(height: 32),
            ElevatedButton.icon(
              onPressed: () {
                context.read<AuthController>().logout();
              },
              icon: const Icon(Icons.logout),
              label: const Text('Log out', style: TextStyle(fontSize: 16)),
              style: ElevatedButton.styleFrom(
                backgroundColor: Colors.redAccent.withValues(alpha: 0.1),
                foregroundColor: Colors.redAccent,
                padding: const EdgeInsets.symmetric(vertical: 16),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _showPasswordDialog(BuildContext context) async {
    final currentPasswordController = TextEditingController();
    final newPasswordController = TextEditingController();
    final currentContext = context;

    return showDialog(
      context: currentContext,
      builder: (dialogContext) {
        bool isSaving = false;

        return StatefulBuilder(
          builder: (builderContext, setDialogState) {
            return AlertDialog(
              title: const Text('Update Password'),
              content: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  TextField(
                    controller: currentPasswordController,
                    obscureText: true,
                    enabled: !isSaving,
                    decoration: const InputDecoration(
                      labelText: 'Current Password',
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: newPasswordController,
                    obscureText: true,
                    enabled: !isSaving,
                    decoration: const InputDecoration(
                      labelText: 'New Password',
                    ),
                  ),
                ],
              ),
              actions: [
                TextButton(
                  onPressed: isSaving
                      ? null
                      : () => Navigator.pop(dialogContext),
                  child: const Text('Cancel'),
                ),
                ElevatedButton(
                  onPressed: isSaving
                      ? null
                      : () async {
                          final currentPwd = currentPasswordController.text;
                          final newPwd = newPasswordController.text;
                          if (currentPwd.isEmpty || newPwd.isEmpty) return;

                          setDialogState(() => isSaving = true);

                          try {
                            await currentContext
                                .read<AuthController>()
                                .updatePassword(currentPwd, newPwd);
                            if (dialogContext.mounted) {
                              Navigator.pop(dialogContext); // Close dialog
                            }
                            if (currentContext.mounted) {
                              UiUtils.showSuccess(
                                currentContext,
                                'Password updated successfully',
                              );
                            }
                          } on ApiException catch (e) {
                            if (dialogContext.mounted) {
                              setDialogState(() => isSaving = false);
                            }
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.message);
                            }
                          } catch (e) {
                            if (dialogContext.mounted) {
                              setDialogState(() => isSaving = false);
                            }
                            if (currentContext.mounted) {
                              UiUtils.showError(currentContext, e.toString());
                            }
                          }
                        },
                  child: isSaving
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Save'),
                ),
              ],
            );
          },
        );
      },
    );
  }
}
