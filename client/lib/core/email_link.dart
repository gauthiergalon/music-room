/// What a link from one of the backend's emails asks the app to do.
enum EmailLinkAction { confirmEmail, resetPassword }

class EmailLink {
  const EmailLink(this.action, this.token);

  final EmailLinkAction action;
  final String token;
}

/// Reads a `musicroom://confirm-email?token=…` or
/// `musicroom://reset-password?token=…` link. On Android, the pages the email
/// links open (served by the backend) offer to continue in the app with
/// these; older emails contained them directly.
///
/// Returns null for anything else.
EmailLink? parseEmailLink(Uri uri) {
  if (uri.scheme != 'musicroom') return null;

  final token = uri.queryParameters['token']?.trim();
  if (token == null || token.isEmpty) return null;

  return switch (uri.host) {
    'confirm-email' => EmailLink(EmailLinkAction.confirmEmail, token),
    'reset-password' => EmailLink(EmailLinkAction.resetPassword, token),
    _ => null,
  };
}
