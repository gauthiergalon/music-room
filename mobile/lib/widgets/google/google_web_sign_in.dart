import 'dart:async';

import 'package:flutter/material.dart';
import 'package:google_sign_in/google_sign_in.dart';
import 'package:provider/provider.dart';

import '../../controllers/auth_controller.dart';
import '../../core/exceptions/api_exception.dart';
import '../../core/logger/logger.dart';
import '../../core/utils/ui_utils.dart';
import 'gsi_button.dart';

/// Google sign-in on the web: the browser SDK only signs in through its own
/// button, and reports the result as an event rather than a return value.
///
/// Each ID token it produces is handed to [onIdToken]; the outcome is shown
/// in a snackbar.
class GoogleWebSignIn extends StatefulWidget {
  const GoogleWebSignIn({
    super.key,
    required this.onIdToken,
    required this.fallbackError,
    this.successMessage,
    this.alignment = Alignment.center,
    this.compact = false,
  });

  final Future<void> Function(String idToken) onIdToken;
  final String fallbackError;
  final String? successMessage;
  final AlignmentGeometry alignment;

  /// Smaller button, to sit in a list row.
  final bool compact;

  @override
  State<GoogleWebSignIn> createState() => _GoogleWebSignInState();
}

class _GoogleWebSignInState extends State<GoogleWebSignIn> {
  late final Future<void> _ready;
  StreamSubscription<String>? _subscription;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    final auth = context.read<AuthController>();
    _ready = auth.ensureGoogleInitialized();
    _subscription = auth.googleButtonIdTokens.listen(
      _onIdToken,
      onError: _onError,
    );
  }

  @override
  void dispose() {
    _subscription?.cancel();
    super.dispose();
  }

  Future<void> _onIdToken(String idToken) async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await widget.onIdToken(idToken);
      final message = widget.successMessage;
      if (mounted && message != null) UiUtils.showSuccess(context, message);
    } on ApiException catch (e) {
      if (mounted) UiUtils.showError(context, e.message);
    } catch (e) {
      if (mounted) UiUtils.showError(context, widget.fallbackError);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _onError(Object error) {
    // Closing Google's popup is not an error worth reporting.
    if (error is GoogleSignInException &&
        error.code == GoogleSignInExceptionCode.canceled) {
      return;
    }
    logger.error('Google sign-in failed', error: error);
    if (mounted) UiUtils.showError(context, widget.fallbackError);
  }

  @override
  Widget build(BuildContext context) {
    final textAlign = widget.alignment == Alignment.center
        ? TextAlign.center
        : TextAlign.start;
    // Same height as the button, so the layout does not jump once it shows.
    final height = widget.compact ? 32.0 : 40.0;

    return FutureBuilder<void>(
      future: _ready,
      builder: (context, snapshot) {
        final Widget child;
        if (snapshot.hasError) {
          final error = snapshot.error;
          child = Text(
            error is ApiException
                ? error.message
                : 'Google sign-in is unavailable.',
            textAlign: textAlign,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          );
        } else if (snapshot.connectionState != ConnectionState.done || _busy) {
          child = SizedBox(
            height: height,
            child: const Center(
              child: SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            ),
          );
        } else {
          child = gsiButton(compact: widget.compact);
        }
        return Align(alignment: widget.alignment, child: child);
      },
    );
  }
}
