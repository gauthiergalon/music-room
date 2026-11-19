import 'package:flutter/material.dart';

import '../../core/exceptions/api_exception.dart';
import '../../core/utils/ui_utils.dart';

/// Runs [action] and reports the outcome in a snackbar.
///
/// Server errors are shown as sent; anything else shows [fallbackError] (or
/// the error itself). Returns whether the action succeeded.
Future<bool> runProfileAction(
  BuildContext context,
  Future<void> Function() action, {
  String? successMessage,
  String? fallbackError,
}) async {
  try {
    await action();
    if (context.mounted && successMessage != null) {
      UiUtils.showSuccess(context, successMessage);
    }
    return true;
  } on ApiException catch (e) {
    if (context.mounted) UiUtils.showError(context, e.message);
  } catch (e) {
    if (context.mounted) {
      UiUtils.showError(context, fallbackError ?? e.toString());
    }
  }
  return false;
}
