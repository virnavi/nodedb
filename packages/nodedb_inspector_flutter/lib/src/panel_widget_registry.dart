import 'package:flutter/material.dart';
import 'package:inspector_sdk/inspector_sdk.dart';

/// Registry that maps panel IDs to Flutter widget builders.
///
/// Pre-populated with built-in panels. Custom plugins can register
/// additional widget factories for their panels.
class PanelWidgetRegistry {
  final Map<String, Widget Function(InspectorPanel)> _factories = {};

  /// Register a widget factory for a panel ID.
  void register(String panelId, Widget Function(InspectorPanel) factory) {
    _factories[panelId] = factory;
  }

  /// Build a widget for a panel. Returns null if no factory is registered.
  Widget? build(InspectorPanel panel) {
    final factory = _factories[panel.descriptor.id];
    return factory?.call(panel);
  }

  /// Whether a widget factory is registered for the given panel ID.
  bool contains(String panelId) => _factories.containsKey(panelId);

  /// Maps icon hint strings to Flutter [IconData].
  static IconData iconFromHint(String hint) {
    const map = <String, IconData>{
      'storage': Icons.storage,
      'hub': Icons.hub,
      'scatter_plot': Icons.scatter_plot,
      'cloud_sync': Icons.cloud_sync,
      'security': Icons.security,
      'verified': Icons.verified,
      'vpn_key': Icons.vpn_key,
      'schema': Icons.schema,
      'flash_on': Icons.flash_on,
      'tune': Icons.tune,
      'settings_applications': Icons.settings_applications,
      'history': Icons.history,
      'psychology': Icons.psychology,
      'dashboard': Icons.dashboard,
      'extension': Icons.extension,
    };
    return map[hint] ?? Icons.extension;
  }
}
