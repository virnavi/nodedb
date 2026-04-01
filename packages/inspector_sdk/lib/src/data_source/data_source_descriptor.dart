/// Metadata for an inspector data source.
class DataSourceDescriptor {
  /// Unique identifier (e.g., 'nosql', 'graph', 'custom-metrics').
  final String id;

  /// Human-readable name.
  final String displayName;

  /// Description of what data this source provides.
  final String? description;

  const DataSourceDescriptor({
    required this.id,
    required this.displayName,
    this.description,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'displayName': displayName,
        if (description != null) 'description': description,
      };
}
