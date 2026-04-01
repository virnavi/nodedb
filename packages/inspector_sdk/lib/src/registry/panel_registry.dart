import '../panel/inspector_panel.dart';

/// Registry for inspector panels.
///
/// Panels are registered by their descriptor ID. Duplicate IDs are rejected.
/// The registry maintains insertion order and supports priority sorting.
class PanelRegistry {
  final Map<String, InspectorPanel> _panels = {};
  final List<void Function(InspectorPanel)> _listeners = [];

  /// Register a panel. Throws [StateError] if a panel with the same ID
  /// already exists.
  void register(InspectorPanel panel) {
    final id = panel.descriptor.id;
    if (_panels.containsKey(id)) {
      throw StateError('Panel "$id" is already registered');
    }
    _panels[id] = panel;
    panel.onRegister();
    for (final listener in _listeners) {
      listener(panel);
    }
  }

  /// Unregister a panel by ID. Returns true if the panel was found.
  bool unregister(String id) {
    final panel = _panels.remove(id);
    if (panel != null) {
      panel.onUnregister();
      return true;
    }
    return false;
  }

  /// Get a panel by ID.
  InspectorPanel? get(String id) => _panels[id];

  /// Get a panel by ID, cast to a specific type.
  T? getAs<T extends InspectorPanel>(String id) {
    final panel = _panels[id];
    return panel is T ? panel : null;
  }

  /// Returns all registered panels, sorted by [PanelDescriptor.sortOrder].
  List<InspectorPanel> get all {
    final sorted = _panels.values.toList()
      ..sort(
          (a, b) => a.descriptor.sortOrder.compareTo(b.descriptor.sortOrder));
    return sorted;
  }

  /// Returns all available (enabled) panels.
  List<InspectorPanel> get available =>
      all.where((p) => p.isAvailable).toList();

  /// Returns descriptors for all registered panels.
  List<Map<String, dynamic>> get descriptors =>
      all.map((p) => p.descriptor.toJson()).toList();

  /// Returns IDs of all available panels.
  List<String> enabledPanelIds() =>
      available.map((p) => p.descriptor.id).toList();

  /// Listen for new panel registrations.
  void addListener(void Function(InspectorPanel) listener) {
    _listeners.add(listener);
  }

  /// Remove a registration listener.
  void removeListener(void Function(InspectorPanel) listener) {
    _listeners.remove(listener);
  }

  /// Number of registered panels.
  int get length => _panels.length;

  /// Whether a panel with the given ID is registered.
  bool contains(String id) => _panels.containsKey(id);
}
