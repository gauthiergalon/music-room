import 'package:flutter/material.dart';

class ProfilePage extends StatefulWidget {
  const ProfilePage({super.key});

  @override
  State<ProfilePage> createState() => _ProfilePageState();
}

class _ProfilePageState extends State<ProfilePage> {
  // Variables d'état pour stocker les informations affichées
  String _username = 'User';
  String _email = 'user@example.com';

  void _showEditDialog(
    String title,
    String currentValue,
    Function(String) onSave, {
    bool isPassword = false,
    bool isEmail = false,
  }) {
    final controller = TextEditingController(
      text: isPassword ? '' : currentValue,
    );
    final confirmController = TextEditingController();
    final formKey = GlobalKey<FormState>();

    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: Text('Edit $title'),
          content: Form(
            key: formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextFormField(
                  controller: controller,
                  obscureText: isPassword,
                  keyboardType: isEmail
                      ? TextInputType.emailAddress
                      : TextInputType.text,
                  decoration: InputDecoration(
                    labelText: title,
                    border: const OutlineInputBorder(),
                  ),
                  validator: (value) {
                    if (value == null || value.isEmpty) {
                      return 'This field cannot be empty';
                    }
                    if (isEmail && !value.contains('@')) {
                      return 'Please enter a valid email address';
                    }
                    return null;
                  },
                ),
                if (isPassword) ...[
                  const SizedBox(height: 16),
                  TextFormField(
                    controller: confirmController,
                    obscureText: true,
                    decoration: const InputDecoration(
                      labelText: 'Confirm Password',
                      border: OutlineInputBorder(),
                    ),
                    validator: (value) {
                      if (value == null || value.isEmpty) {
                        return 'Confirm password cannot be empty';
                      }
                      if (value != controller.text) {
                        return 'Passwords do not match';
                      }
                      return null;
                    },
                  ),
                ],
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: () {
                if (formKey.currentState!.validate()) {
                  onSave(controller.text);
                  Navigator.pop(context);
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('$title updated successfully')),
                  );
                }
              },
              child: const Text('Save'),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.symmetric(vertical: 24.0, horizontal: 16.0),
          children: [
            Card(
              elevation: 2,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              child: Column(
                children: [
                  ListTile(
                    leading: const Icon(Icons.person_outline),
                    title: const Text('Username'),
                    subtitle: Text(_username),
                    trailing: const Icon(Icons.edit, size: 20),
                    onTap: () {
                      _showEditDialog('Username', _username, (newValue) {
                        setState(() => _username = newValue);
                      });
                    },
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.email_outlined),
                    title: const Text('Email Address'),
                    subtitle: Text(_email),
                    trailing: const Icon(Icons.edit, size: 20),
                    onTap: () {
                      _showEditDialog('Email Address', _email, (newValue) {
                        setState(() => _email = newValue);
                      }, isEmail: true);
                    },
                  ),
                  const Divider(height: 1),
                  ListTile(
                    leading: const Icon(Icons.lock_outline),
                    title: const Text('Password'),
                    subtitle: const Text('********'),
                    trailing: const Icon(Icons.edit, size: 20),
                    onTap: () {
                      _showEditDialog('Password', '', (newValue) {
                        // TODO: Implémenter la logique de mise à jour du mot de passe avec le backend
                      }, isPassword: true);
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
