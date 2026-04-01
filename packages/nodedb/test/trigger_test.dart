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
    tempDir = Directory.systemTemp.createTempSync('nodedb_trigger_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('Triggers', () {
    test('registerTrigger returns a positive trigger ID', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final triggerId = engine.registerTrigger(
        collection: 'users',
        event: 'insert',
        timing: 'after',
      );

      expect(triggerId, greaterThan(0));
      engine.close();
    });

    test('registerTrigger with name', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final triggerId = engine.registerTrigger(
        collection: 'users',
        event: 'update',
        timing: 'before',
        name: 'audit_log_trigger',
      );

      expect(triggerId, greaterThan(0));

      final triggers = engine.listTriggers();
      final match = triggers.where((t) => t['name'] == 'audit_log_trigger');
      expect(match, hasLength(1));

      engine.close();
    });

    test('listTriggers returns registered triggers', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.registerTrigger(
        collection: 'orders',
        event: 'insert',
        timing: 'after',
        name: 'order_trigger',
      );
      engine.registerTrigger(
        collection: 'users',
        event: 'delete',
        timing: 'before',
        name: 'user_delete_trigger',
      );

      final triggers = engine.listTriggers();
      expect(triggers.length, greaterThanOrEqualTo(2));

      final names = triggers.map((t) => t['name']).toList();
      expect(names, contains('order_trigger'));
      expect(names, contains('user_delete_trigger'));

      engine.close();
    });

    test('unregisterTrigger removes a trigger', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final triggerId = engine.registerTrigger(
        collection: 'items',
        event: 'insert',
        timing: 'after',
        name: 'to_remove',
      );

      expect(engine.unregisterTrigger(triggerId), isTrue);

      final triggers = engine.listTriggers();
      final match = triggers.where((t) => t['name'] == 'to_remove');
      expect(match, isEmpty);

      engine.close();
    });

    test('unregisterTrigger returns false for unknown ID', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      expect(engine.unregisterTrigger(99999), isFalse);
      engine.close();
    });

    test('setTriggerEnabled disables and re-enables a trigger', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final triggerId = engine.registerTrigger(
        collection: 'users',
        event: 'insert',
        timing: 'after',
        name: 'toggle_trigger',
      );

      // Verify initially enabled
      var triggers = engine.listTriggers();
      var t = triggers.firstWhere((t) => t['name'] == 'toggle_trigger');
      expect(t['enabled'], isTrue);

      // Disable
      expect(engine.setTriggerEnabled(triggerId, enabled: false), isTrue);
      triggers = engine.listTriggers();
      t = triggers.firstWhere((t) => t['name'] == 'toggle_trigger');
      expect(t['enabled'], isFalse);

      // Re-enable
      expect(engine.setTriggerEnabled(triggerId, enabled: true), isTrue);
      triggers = engine.listTriggers();
      t = triggers.firstWhere((t) => t['name'] == 'toggle_trigger');
      expect(t['enabled'], isTrue);

      engine.close();
    });

    test('setTriggerEnabled returns false for unknown ID', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);
      expect(engine.setTriggerEnabled(99999, enabled: false), isFalse);
      engine.close();
    });

    test('registerMeshTrigger returns a positive trigger ID', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final triggerId = engine.registerMeshTrigger(
        sourceDatabase: 'remote_db',
        collection: 'shared_items',
        event: 'insert',
        name: 'mesh_sync',
      );

      expect(triggerId, greaterThan(0));
      engine.close();
    });

    test('multiple triggers on same collection', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final id1 = engine.registerTrigger(
        collection: 'users',
        event: 'insert',
        timing: 'before',
        name: 'before_insert',
      );
      final id2 = engine.registerTrigger(
        collection: 'users',
        event: 'insert',
        timing: 'after',
        name: 'after_insert',
      );

      expect(id1, isNot(equals(id2)));

      final triggers = engine.listTriggers();
      expect(triggers.length, greaterThanOrEqualTo(2));

      engine.close();
    });

    test('trigger fields in listTriggers are correct', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.registerTrigger(
        collection: 'orders',
        event: 'update',
        timing: 'after',
        name: 'check_fields',
      );

      final triggers = engine.listTriggers();
      final t = triggers.firstWhere((t) => t['name'] == 'check_fields');

      expect(t['id'], isA<int>());
      expect(t['event'], 'update');
      expect(t['timing'], 'after');
      expect(t['enabled'], isTrue);
      expect(t['name'], 'check_fields');

      engine.close();
    });
  });
}
