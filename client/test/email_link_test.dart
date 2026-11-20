import 'package:flutter_test/flutter_test.dart';
import 'package:mobile/core/email_link.dart';

void main() {
  test('reads the musicroom:// links', () {
    final confirm = parseEmailLink(
      Uri.parse('musicroom://confirm-email?token=abc'),
    );
    expect(confirm?.action, EmailLinkAction.confirmEmail);
    expect(confirm?.token, 'abc');

    final reset = parseEmailLink(
      Uri.parse('musicroom://reset-password?token=def'),
    );
    expect(reset?.action, EmailLinkAction.resetPassword);
    expect(reset?.token, 'def');
  });

  test('ignores other links', () {
    for (final url in [
      'musicroom://confirm-email',
      'musicroom://confirm-email?token=',
      'musicroom://unknown?token=abc',
      'https://api.example.com/confirm-email?token=abc',
    ]) {
      expect(parseEmailLink(Uri.parse(url)), isNull, reason: url);
    }
  });
}
