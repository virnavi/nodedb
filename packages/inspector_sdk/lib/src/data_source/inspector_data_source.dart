import 'data_source_descriptor.dart';

/// Abstract data source that panels can consume.
///
/// A data source provides raw data access that one or more panels may use.
/// This allows custom panels to be built on top of existing data sources,
/// or entirely new data sources to be registered.
abstract class InspectorDataSource {
  /// Metadata describing this data source.
  DataSourceDescriptor get descriptor;

  /// Whether this data source is currently connected and available.
  bool get isConnected;

  /// Query the data source with the given parameters.
  /// Returns JSON-serializable data.
  dynamic query(Map<String, dynamic> params);

  /// Returns a summary of the data source state.
  Map<String, dynamic> stats();
}
