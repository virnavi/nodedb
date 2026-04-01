import 'dart:io';

import 'package:nodedb/nodedb.dart';
/// Helper for creating temporary NodeDB instances in tests.
class TestNodeDB {
  static final List<String> _tempDirs = [];

  /// Create a temporary NodeDB for testing.
  ///
  /// All optional engines default to disabled. Pass `true` to enable
  /// specific engines for your test.
  static NodeDB create({
    bool graph = false,
    VectorOpenConfig? vector,
    bool dac = false,
    bool provenance = false,
    bool keyResolver = false,
    DatabaseMesh? mesh,
    String databaseName = 'test',
  }) {
    final dir = Directory.systemTemp.createTempSync('nodedb_test_');
    _tempDirs.add(dir.path);

    return NodeDB.open(
      directory: dir.path,
      databaseName: databaseName,
      mesh: mesh,
      graphEnabled: graph,
      vectorConfig: vector,
      dacEnabled: dac,
      provenanceEnabled: provenance,
      keyResolverEnabled: keyResolver,
    );
  }

  /// Create a temporary directory path for testing.
  static String tempDir([String prefix = 'nodedb_test_']) {
    final dir = Directory.systemTemp.createTempSync(prefix);
    _tempDirs.add(dir.path);
    return dir.path;
  }

  /// Clean up all temporary directories created during tests.
  static void cleanUp() {
    for (final path in _tempDirs) {
      try {
        Directory(path).deleteSync(recursive: true);
      } catch (_) {}
    }
    _tempDirs.clear();
  }
}
