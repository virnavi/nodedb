import 'panel_descriptor.dart';

/// Abstract interface that all inspector panels must implement.
///
/// Panels provide:
/// 1. A [descriptor] declaring metadata and supported actions.
/// 2. A [summary] for snapshot aggregation.
/// 3. A [dispatch] method for action-based command routing.
/// 4. An [isAvailable] flag indicating whether the panel's backing data
///    source is active.
abstract class InspectorPanel {
  /// Metadata describing this panel.
  PanelDescriptor get descriptor;

  /// Whether this panel is currently available.
  /// Returns false if the backing engine/data source is not configured.
  bool get isAvailable;

  /// Returns a summary map for snapshot aggregation.
  /// Must return only JSON-serializable values.
  Map<String, dynamic> summary();

  /// Dispatch an action by name with the given parameters.
  /// Returns a JSON-serializable result.
  ///
  /// Throws [ArgumentError] if the action is unknown.
  /// Throws [StateError] if the panel is not available.
  dynamic dispatch(String action, Map<String, dynamic> params);

  /// Lifecycle hook called when the panel is registered.
  void onRegister() {}

  /// Lifecycle hook called when the panel is unregistered.
  void onUnregister() {}
}
