/// Describes a database migration to a target version.
class NodeMigration {
  /// The version this migration targets.
  final int toVersion;

  /// Callback that populates a [MigrationContext] with operations.
  final void Function(MigrationContext ctx) migrate;

  const NodeMigration({required this.toVersion, required this.migrate});
}

/// Builder that collects migration operations.
class MigrationContext {
  final List<Map<String, dynamic>> _operations = [];

  /// Rename a collection (sled tree) from [from] to [to].
  void renameCollection(String from, String to) {
    _operations.add({'type': 'rename_tree', 'from': from, 'to': to});
  }

  /// Drop a collection (sled tree) and all its data.
  void dropCollection(String name) {
    _operations.add({'type': 'drop_tree', 'name': name});
  }

  /// The collected operations (internal use).
  List<Map<String, dynamic>> get operations => List.unmodifiable(_operations);
}

/// Result of a successful migration.
class MigrationResult {
  final String status;
  final int version;

  const MigrationResult({required this.status, required this.version});

  @override
  String toString() => 'MigrationResult(status: $status, version: $version)';
}
