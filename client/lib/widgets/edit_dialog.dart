import 'package:flutter/material.dart';

Future<void> showEditDialog(
  BuildContext context, {
  required String title,
  required String currentValue,
  required Future<void> Function(String) onSave,
  bool isPassword = false,
  bool isEmail = false,
}) {
  return showDialog<void>(
    context: context,
    builder: (_) => _EditDialog(
      title: title,
      currentValue: currentValue,
      onSave: onSave,
      isPassword: isPassword,
      isEmail: isEmail,
    ),
  );
}

class _EditDialog extends StatefulWidget {
  const _EditDialog({
    required this.title,
    required this.currentValue,
    required this.onSave,
    required this.isPassword,
    required this.isEmail,
  });

  final String title;
  final String currentValue;
  final Future<void> Function(String) onSave;
  final bool isPassword;
  final bool isEmail;

  @override
  State<_EditDialog> createState() => _EditDialogState();
}

class _EditDialogState extends State<_EditDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.isPassword ? '' : widget.currentValue,
  );
  final _confirmController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _isSaving = false;

  @override
  void dispose() {
    _controller.dispose();
    _confirmController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isSaving = true);
    try {
      await widget.onSave(_controller.text);
      if (mounted) Navigator.pop(context);
    } finally {
      if (mounted) setState(() => _isSaving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
      titleTextStyle: const TextStyle(
        fontSize: 20,
        fontWeight: FontWeight.w600,
      ),
      actionsPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      title: Text('Edit ${widget.title}'),
      content: Form(
        key: _formKey,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextFormField(
              controller: _controller,
              obscureText: widget.isPassword,
              enabled: !_isSaving,
              keyboardType: widget.isEmail
                  ? TextInputType.emailAddress
                  : TextInputType.text,
              decoration: InputDecoration(
                labelText: widget.title,
                border: const OutlineInputBorder(),
              ),
              validator: (value) {
                if (value == null || value.isEmpty) {
                  return 'This field cannot be empty';
                }
                if (widget.isEmail && !value.contains('@')) {
                  return 'Please enter a valid email address';
                }
                return null;
              },
            ),
            if (widget.isPassword) ...[
              const SizedBox(height: 16),
              TextFormField(
                controller: _confirmController,
                obscureText: true,
                enabled: !_isSaving,
                decoration: const InputDecoration(
                  labelText: 'Confirm Password',
                  border: OutlineInputBorder(),
                ),
                validator: (value) {
                  if (value == null || value.isEmpty) {
                    return 'Confirm password cannot be empty';
                  }
                  if (value != _controller.text) {
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
          onPressed: _isSaving ? null : () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: _isSaving ? null : _save,
          child: _isSaving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Save'),
        ),
      ],
    );
  }
}
