import 'panel_action.dart';

/// Describes an inspector panel's identity and capabilities.
class PanelDescriptor {
  /// Unique identifier used as the routing key (e.g., 'nosql', 'my-panel').
  final String id;

  /// Human-readable display name (e.g., 'NoSQL', 'My Panel').
  final String displayName;

  /// Optional longer description for tooltips / help text.
  final String? description;

  /// Icon hint for UI rendering. A string identifier that consumers map to
  /// a platform-specific icon. Built-in hints: 'storage', 'hub',
  /// 'scatter_plot', 'cloud_sync', 'security', 'verified', 'vpn_key',
  /// 'schema', 'flash_on', 'tune', 'settings_applications', 'history',
  /// 'psychology', 'dashboard', 'extension'.
  final String iconHint;

  /// The list of actions this panel supports.
  final List<PanelAction> actions;

  /// Sort order hint (lower = earlier in navigation). Default 100.
  final int sortOrder;

  /// Category for grouping: 'data', 'security', 'system', 'ai', 'custom'.
  final String category;

  const PanelDescriptor({
    required this.id,
    required this.displayName,
    this.description,
    this.iconHint = 'extension',
    this.actions = const [],
    this.sortOrder = 100,
    this.category = 'custom',
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'displayName': displayName,
        if (description != null) 'description': description,
        'iconHint': iconHint,
        'sortOrder': sortOrder,
        'category': category,
        if (actions.isNotEmpty)
          'actions': actions.map((a) => a.toJson()).toList(),
      };
}
