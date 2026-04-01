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
    tempDir = Directory.systemTemp.createTempSync('nodedb_singleton_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('Singleton', () {
    test('singletonCreate creates a document with defaults', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final doc = engine.singletonCreate('app_config', {
        'theme': 'light',
        'locale': 'en',
      });

      expect(doc.id, 1);
      expect(doc.data['theme'], 'light');
      expect(doc.data['locale'], 'en');

      engine.close();
    });

    test('singletonGet retrieves the singleton document', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.singletonCreate('app_config', {
        'theme': 'dark',
        'fontSize': 14,
      });

      final doc = engine.singletonGet('app_config');
      expect(doc.id, 1);
      expect(doc.data['theme'], 'dark');
      expect(doc.data['fontSize'], 14);

      engine.close();
    });

    test('singletonPut updates the singleton document', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.singletonCreate('app_config', {'theme': 'light'});

      final updated = engine.singletonPut('app_config', {
        'theme': 'dark',
        'newField': true,
      });

      expect(updated.id, 1);
      expect(updated.data['theme'], 'dark');
      expect(updated.data['newField'], true);

      // Verify persistence
      final fetched = engine.singletonGet('app_config');
      expect(fetched.data['theme'], 'dark');

      engine.close();
    });

    test('singletonReset restores defaults', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.singletonCreate('app_config', {'theme': 'light', 'locale': 'en'});
      engine.singletonPut('app_config', {'theme': 'dark', 'locale': 'fr'});

      final reset = engine.singletonReset('app_config');
      expect(reset.id, 1);
      expect(reset.data['theme'], 'light');
      expect(reset.data['locale'], 'en');

      engine.close();
    });

    test('isSingleton returns true for singleton collections', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.singletonCreate('app_config', {'key': 'value'});
      expect(engine.isSingleton('app_config'), isTrue);

      engine.close();
    });

    test('isSingleton returns false for regular collections', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.writeTxn([WriteOp.put('users', data: {'name': 'Alice'})]);
      expect(engine.isSingleton('users'), isFalse);

      engine.close();
    });

    test('singletonCreate is idempotent', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final doc1 = engine.singletonCreate('app_config', {'theme': 'light'});
      engine.singletonPut('app_config', {'theme': 'dark'});

      // Re-creating should not overwrite the current data
      final doc2 = engine.singletonCreate('app_config', {'theme': 'light'});
      expect(doc2.id, doc1.id);

      engine.close();
    });

    test('singleton persists across reopen', () {
      final path = '${tempDir.path}/persist';
      Directory(path).createSync();

      var engine = NoSqlEngine.open(bindings, path);
      engine.singletonCreate('settings', {'volume': 50});
      engine.singletonPut('settings', {'volume': 75});
      engine.close();

      engine = NoSqlEngine.open(bindings, path);
      final doc = engine.singletonGet('settings');
      expect(doc.data['volume'], 75);
      expect(engine.isSingleton('settings'), isTrue);
      engine.close();
    });
  });

  group('NodeDB facade — Singleton', () {
    test('delegates singleton operations', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      final created = db.singletonCreate('config', {'key': 'val'});
      expect(created.id, 1);

      final fetched = db.singletonGet('config');
      expect(fetched.data['key'], 'val');

      final updated = db.singletonPut('config', {'key': 'new'});
      expect(updated.data['key'], 'new');

      expect(db.isSingleton('config'), isTrue);

      final reset = db.singletonReset('config');
      expect(reset.data['key'], 'val');

      db.close();
    });
  });
}
