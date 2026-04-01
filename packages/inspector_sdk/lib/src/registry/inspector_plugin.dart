import 'panel_registry.dart';
import 'data_source_registry.dart';

/// Interface for inspector plugins that register panels and data sources.
///
/// Plugins bundle related panels and data sources together for clean
/// registration and cleanup.
abstract class InspectorPlugin {
  /// Unique plugin identifier.
  String get id;

  /// Human-readable plugin name.
  String get name;

  /// Register this plugin's panels and data sources.
  void register(PanelRegistry panels, DataSourceRegistry dataSources);

  /// Unregister this plugin's panels and data sources.
  void unregister(PanelRegistry panels, DataSourceRegistry dataSources);
}
