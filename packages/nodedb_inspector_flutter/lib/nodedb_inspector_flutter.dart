/// Flutter debug inspector overlay for NodeDB databases.
///
/// Provides a [NodeInspectorOverlay] widget that wraps your app and
/// adds a floating debug button. Tapping it opens a full inspector
/// screen with panel tabs for every NodeDB engine.
///
/// ```dart
/// NodeInspectorOverlay(
///   db: myNodeDB,
///   enabled: kDebugMode,
///   child: MyApp(),
/// )
/// ```
library;

export 'src/inspector_overlay.dart';
export 'src/inspector_screen.dart';
export 'src/inspector_theme.dart';
export 'src/panel_widget_registry.dart';
export 'src/panels/panel_helpers.dart';
