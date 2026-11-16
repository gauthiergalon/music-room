import 'package:flutter/widgets.dart';
import 'package:google_sign_in_web/web_only.dart' as gsi;

/// Google's own sign-in button, rendered by its SDK. [compact] fits it in a
/// list row.
Widget gsiButton({bool compact = false}) => gsi.renderButton(
  configuration: gsi.GSIButtonConfiguration(
    theme: gsi.GSIButtonTheme.filledBlack,
    size: compact ? gsi.GSIButtonSize.medium : gsi.GSIButtonSize.large,
    text: gsi.GSIButtonText.continueWith,
    shape: gsi.GSIButtonShape.pill,
    logoAlignment: gsi.GSIButtonLogoAlignment.left,
  ),
);
