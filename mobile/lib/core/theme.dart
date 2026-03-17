import 'package:flutter/material.dart';

class AppTheme {
  static const double spacingSm = 8.0;
  static const double spacingMd = 16.0;
  static const double spacingLg = 24.0;

  static const EdgeInsets paddingSm = EdgeInsets.all(spacingSm);
  static const EdgeInsets paddingMd = EdgeInsets.all(spacingMd);
  static const EdgeInsets paddingLg = EdgeInsets.all(spacingLg);

  static const EdgeInsets screenPadding = EdgeInsets.symmetric(
    vertical: spacingLg,
    horizontal: spacingMd,
  );

  static ThemeData get darkTheme {
    final cs = ColorScheme.fromSeed(
      seedColor: Colors.deepPurple,
      brightness: Brightness.dark,
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: cs,
      cardTheme: CardThemeData(
        color: cs.secondaryContainer,
        elevation: 2,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: cs.secondaryContainer,
          foregroundColor: cs.onSecondaryContainer,
        ),
      ),
      searchBarTheme: SearchBarThemeData(
        backgroundColor: WidgetStatePropertyAll<Color?>(cs.secondaryContainer),
        elevation: WidgetStatePropertyAll<double>(0),
      ),
      listTileTheme: const ListTileThemeData(
        contentPadding: EdgeInsets.symmetric(
          horizontal: spacingMd,
          vertical: spacingSm,
        ),
      ),
      dividerTheme: DividerThemeData(thickness: 1),
      floatingActionButtonTheme: FloatingActionButtonThemeData(
        backgroundColor: cs.secondaryContainer,
        foregroundColor: cs.onSecondaryContainer,
      ),
    );
  }
}
