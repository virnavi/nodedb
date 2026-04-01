import '../data_source/inspector_data_source.dart';

/// Registry for inspector data sources.
class DataSourceRegistry {
  final Map<String, InspectorDataSource> _sources = {};

  /// Register a data source. Throws [StateError] if a source with the same
  /// ID already exists.
  void register(InspectorDataSource source) {
    final id = source.descriptor.id;
    if (_sources.containsKey(id)) {
      throw StateError('Data source "$id" is already registered');
    }
    _sources[id] = source;
  }

  /// Unregister a data source by ID. Returns true if found.
  bool unregister(String id) => _sources.remove(id) != null;

  /// Get a data source by ID.
  InspectorDataSource? get(String id) => _sources[id];

  /// Get a data source by ID, cast to a specific type.
  T? getAs<T extends InspectorDataSource>(String id) {
    final source = _sources[id];
    return source is T ? source : null;
  }

  /// All registered data sources.
  List<InspectorDataSource> get all => _sources.values.toList();

  /// All connected (available) data sources.
  List<InspectorDataSource> get connected =>
      all.where((s) => s.isConnected).toList();

  /// Whether a data source with the given ID is registered.
  bool contains(String id) => _sources.containsKey(id);

  /// Number of registered data sources.
  int get length => _sources.length;
}
