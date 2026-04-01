import 'package:flutter/material.dart';

/// Dark theme colors matching the web inspector dashboard.
abstract final class InspectorColors {
  static const bg = Color(0xFF1a1b26);
  static const surface = Color(0xFF24283b);
  static const surfaceAlt = Color(0xFF1f2335);
  static const border = Color(0xFF3b4261);
  static const text = Color(0xFFc0caf5);
  static const textDim = Color(0xFF565f89);
  static const accent = Color(0xFF7aa2f7);
  static const green = Color(0xFF9ece6a);
  static const red = Color(0xFFf7768e);
  static const yellow = Color(0xFFe0af68);
  static const magenta = Color(0xFFbb9af7);
  static const cyan = Color(0xFF7dcfff);
}

/// Builds the inspector dark theme.
ThemeData inspectorTheme() {
  return ThemeData.dark(useMaterial3: true).copyWith(
    scaffoldBackgroundColor: InspectorColors.bg,
    colorScheme: const ColorScheme.dark(
      primary: InspectorColors.accent,
      secondary: InspectorColors.cyan,
      surface: InspectorColors.surface,
      error: InspectorColors.red,
      onPrimary: InspectorColors.bg,
      onSecondary: InspectorColors.bg,
      onSurface: InspectorColors.text,
      onError: InspectorColors.bg,
    ),
    cardTheme: const CardThemeData(
      color: InspectorColors.surface,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(8)),
        side: BorderSide(color: InspectorColors.border, width: 1),
      ),
    ),
    dividerColor: InspectorColors.border,
    navigationRailTheme: const NavigationRailThemeData(
      backgroundColor: InspectorColors.surfaceAlt,
      selectedIconTheme: IconThemeData(color: InspectorColors.accent),
      unselectedIconTheme: IconThemeData(color: InspectorColors.textDim),
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: InspectorColors.surfaceAlt,
      foregroundColor: InspectorColors.text,
      elevation: 0,
    ),
    textTheme: const TextTheme(
      bodyLarge: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      bodyMedium: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      bodySmall: TextStyle(color: InspectorColors.textDim, fontFamily: 'monospace'),
      titleLarge: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      titleMedium: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      titleSmall: TextStyle(color: InspectorColors.textDim, fontFamily: 'monospace'),
      labelLarge: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      labelMedium: TextStyle(color: InspectorColors.text, fontFamily: 'monospace'),
      labelSmall: TextStyle(color: InspectorColors.textDim, fontFamily: 'monospace'),
    ),
  );
}
