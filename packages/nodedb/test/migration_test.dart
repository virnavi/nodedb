import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:test/test.dart';

void main() {
  late NodeDbBindings bindings;
  late Directory tempDir;

  setUpAll(() {
    bindings = NodeDbBindings(loadNodeDbLibrary());
  });

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('nodedb_migration_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('MigrationContext', () {
    test('collects rename operations', () {
      final ctx = MigrationContext();
      ctx.renameCollection('old_users', 'new_users');
      expect(ctx.operations, hasLength(1));
      expect(ctx.operations[0]['type'], 'rename_tree');
      expect(ctx.operations[0]['from'], 'old_users');
      expect(ctx.operations[0]['to'], 'new_users');
    });

    test('collects drop operations', () {
      final ctx = MigrationContext();
      ctx.dropCollection('temp');
      expect(ctx.operations, hasLength(1));
      expect(ctx.operations[0]['type'], 'drop_tree');
      expect(ctx.operations[0]['name'], 'temp');
    });

    test('collects multiple operations in order', () {
      final ctx = MigrationContext();
      ctx.renameCollection('a', 'b');
      ctx.dropCollection('c');
      ctx.renameCollection('d', 'e');
      expect(ctx.operations, hasLength(3));
      expect(ctx.operations[0]['type'], 'rename_tree');
      expect(ctx.operations[1]['type'], 'drop_tree');
      expect(ctx.operations[2]['type'], 'rename_tree');
    });
  });

  group('runMigration', () {
    test('runs successfully and returns result', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final result = engine.runMigration(NodeMigration(
        toVersion: 2,
        migrate: (ctx) {
          ctx.renameCollection('old_name', 'new_name');
        },
      ));

      expect(result.status, 'migrated');
      expect(result.version, 2);
      engine.close();
    });

    test('drop tree operation succeeds', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final result = engine.runMigration(NodeMigration(
        toVersion: 2,
        migrate: (ctx) {
          ctx.dropCollection('nonexistent');
        },
      ));

      expect(result.status, 'migrated');
      expect(result.version, 2);
      engine.close();
    });

    test('is idempotent — re-running lower version is no-op', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      // Migrate to v3
      final result1 = engine.runMigration(NodeMigration(
        toVersion: 3,
        migrate: (ctx) {
          // No-op migration
        },
      ));
      expect(result1.version, 3);

      // Try running v2 migration — should be skipped (version already >= 3)
      final result2 = engine.runMigration(NodeMigration(
        toVersion: 2,
        migrate: (ctx) {
          ctx.dropCollection('anything');
        },
      ));

      // Result should still show current version
      expect(result2.status, 'migrated');
      engine.close();
    });

    test('NodeDB.runMigration delegates to nosql', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      final result = db.runMigration(NodeMigration(
        toVersion: 1,
        migrate: (ctx) {
          // Empty migration
        },
      ));

      expect(result.status, 'migrated');
      expect(result.version, 1);
      db.close();
    });
  });
}
