import 'package:flutter/material.dart';
import '../core/theme.dart';
import '../widgets/edit_dialog.dart';
import '../widgets/profile_info_tile.dart';

class ProfilePage extends StatefulWidget {
  const ProfilePage({super.key});

  @override
  State<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  // Variables d'état pour stocker les informations affichées
  String _username = 'User';
  String _email = 'user@example.com';

  @override
  Widget build(BuildContext context) {
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
                      subtitle: _username,
                      onTap: () {
                        showEditDialog(
                          context,
                          title: 'Username',
                          currentValue: _username,
                          onSave: (newValue) =>
                              setState(() => _username = newValue),
                        );
                      },
                    ),
                    const Divider(),
                    ProfileInfoTile(
                      icon: Icons.email_outlined,
                      title: 'Email Address',
                      subtitle: _email,
                      onTap: () {
                        showEditDialog(
                          context,
                          title: 'Email Address',
                          currentValue: _email,
                          isEmail: true,
                          onSave: (newValue) =>
                              setState(() => _email = newValue),
                        );
                      },
                    ),
                    const Divider(),
                    ProfileInfoTile(
                      icon: Icons.lock_outline,
                      title: 'Password',
                      subtitle: '********',
                      onTap: () {
                        showEditDialog(
                          context,
                          title: 'Password',
                          currentValue: '',
                          isPassword: true,
                          onSave: (newValue) {
                            // TODO: Implémenter la logique de mise à jour du mot de passe avec le backend
                          },
                        );
                      },
                    ),
                  ],
                ),
              ),
            ],
          ),
      ),
    );
  }
}
