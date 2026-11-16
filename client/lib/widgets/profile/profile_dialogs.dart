import 'package:flutter/material.dart';

/// Saves what the dialog collected. Returns `true` to close the dialog, or
/// `false` to keep it open (the caller has already reported the error).
typedef DialogSave<T> = Future<bool> Function(T value);

Future<void> showPasswordDialog(
  BuildContext context, {
  required DialogSave<({String current, String next})> onSave,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => _PasswordDialog(onSave: onSave),
  );
}

Future<void> showFavoriteGenresDialog(
  BuildContext context, {
  required List<String> current,
  required DialogSave<List<String>> onSave,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => _FavoriteGenresDialog(current: current, onSave: onSave),
  );
}

/// Returns the chosen privacy level, or `null` when cancelled.
Future<String?> showPrivacyDialog(BuildContext context, String current) {
  return showDialog<String>(
    context: context,
    builder: (_) => _PrivacyDialog(current: current),
  );
}

Widget _saveButtonChild(bool isSaving) {
  return isSaving
      ? const SizedBox(
          width: 16,
          height: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        )
      : const Text('Save');
}

class _PasswordDialog extends StatefulWidget {
  const _PasswordDialog({required this.onSave});

  final DialogSave<({String current, String next})> onSave;

  @override
  State<_PasswordDialog> createState() => _PasswordDialogState();
}

class _PasswordDialogState extends State<_PasswordDialog> {
  final _currentController = TextEditingController();
  final _newController = TextEditingController();
  bool _isSaving = false;

  @override
  void dispose() {
    _currentController.dispose();
    _newController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final current = _currentController.text;
    final next = _newController.text;
    if (current.isEmpty || next.isEmpty) return;

    setState(() => _isSaving = true);
    final saved = await widget.onSave((current: current, next: next));
    if (!mounted) return;
    if (saved) {
      Navigator.pop(context);
    } else {
      setState(() => _isSaving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Update Password'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _currentController,
            obscureText: true,
            enabled: !_isSaving,
            decoration: const InputDecoration(labelText: 'Current Password'),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _newController,
            obscureText: true,
            enabled: !_isSaving,
            decoration: const InputDecoration(labelText: 'New Password'),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: _isSaving ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: _isSaving ? null : _save,
          child: _saveButtonChild(_isSaving),
        ),
      ],
    );
  }
}

class _FavoriteGenresDialog extends StatefulWidget {
  const _FavoriteGenresDialog({required this.current, required this.onSave});

  final List<String> current;
  final DialogSave<List<String>> onSave;

  @override
  State<_FavoriteGenresDialog> createState() => _FavoriteGenresDialogState();
}

class _FavoriteGenresDialogState extends State<_FavoriteGenresDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.current.join(', '),
  );
  bool _isSaving = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final genres = _controller.text
        .split(',')
        .map((genre) => genre.trim())
        .where((genre) => genre.isNotEmpty)
        .toSet()
        .toList();

    setState(() => _isSaving = true);
    final saved = await widget.onSave(genres);
    if (!mounted) return;
    if (saved) {
      Navigator.pop(context);
    } else {
      setState(() => _isSaving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Music Tastes'),
      content: TextField(
        controller: _controller,
        enabled: !_isSaving,
        decoration: const InputDecoration(hintText: 'house, techno, jazz'),
      ),
      actions: [
        TextButton(
          onPressed: _isSaving ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: _isSaving ? null : _save,
          child: _saveButtonChild(_isSaving),
        ),
      ],
    );
  }
}

class _PrivacyDialog extends StatefulWidget {
  const _PrivacyDialog({required this.current});

  final String current;

  @override
  State<_PrivacyDialog> createState() => _PrivacyDialogState();
}

class _PrivacyDialogState extends State<_PrivacyDialog> {
  late String _selected = widget.current;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Profile Privacy'),
      content: RadioGroup<String>(
        groupValue: _selected,
        onChanged: (value) {
          if (value != null) setState(() => _selected = value);
        },
        child: const Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            RadioListTile<String>(
              value: 'Public',
              title: Text('Public'),
              subtitle: Text('Everyone can see your profile genres'),
            ),
            RadioListTile<String>(
              value: 'Friends',
              title: Text('Friends'),
              subtitle: Text('Only friends can see your genres'),
            ),
            RadioListTile<String>(
              value: 'Private',
              title: Text('Private'),
              subtitle: Text('Only you can see your genres'),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: () => Navigator.pop(context, _selected),
          child: const Text('Save'),
        ),
      ],
    );
  }
}
