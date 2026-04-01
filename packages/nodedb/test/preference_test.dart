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
    tempDir = Directory.systemTemp.createTempSync('nodedb_preference_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('Preferences', () {
    test('prefSet and prefGet round-trip a string value', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'locale', 'en');

      final resp = engine.prefGet('app_prefs', 'locale');
      expect(resp is Map, isTrue);
      expect(resp['found'], isTrue);
      expect(resp['value'], 'en');

      engine.close();
    });

    test('prefSet and prefGet round-trip numeric value', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'font_size', 16);

      final resp = engine.prefGet('app_prefs', 'font_size');
      expect(resp['found'], isTrue);
      expect(resp['value'], 16);

      engine.close();
    });

    test('prefSet and prefGet round-trip boolean value', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'dark_mode', true);

      final resp = engine.prefGet('app_prefs', 'dark_mode');
      expect(resp['found'], isTrue);
      expect(resp['value'], isTrue);

      engine.close();
    });

    test('prefGet returns found=false for missing key', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      final resp = engine.prefGet('app_prefs', 'nonexistent');
      expect(resp is Map, isTrue);
      expect(resp['found'], isFalse);

      engine.close();
    });

    test('prefKeys lists all keys in a store', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'locale', 'en');
      engine.prefSet('app_prefs', 'theme', 'dark');
      engine.prefSet('app_prefs', 'font_size', 14);

      final keys = engine.prefKeys('app_prefs');
      expect(keys, unorderedEquals(['locale', 'theme', 'font_size']));

      engine.close();
    });

    test('prefRemove deletes an existing key', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'locale', 'en');
      expect(engine.prefRemove('app_prefs', 'locale'), isTrue);

      final resp = engine.prefGet('app_prefs', 'locale');
      expect(resp['found'], isFalse);

      engine.close();
    });

    test('prefRemove returns false for missing key', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      expect(engine.prefRemove('app_prefs', 'nonexistent'), isFalse);

      engine.close();
    });

    test('prefSet overwrites existing value', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'locale', 'en');
      engine.prefSet('app_prefs', 'locale', 'fr');

      final resp = engine.prefGet('app_prefs', 'locale');
      expect(resp['value'], 'fr');

      engine.close();
    });

    test('prefShareable returns shareable entries', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('app_prefs', 'locale', 'en', shareable: true);
      engine.prefSet('app_prefs', 'secret', 'hidden', shareable: false);

      final entries = engine.prefShareable('app_prefs');
      expect(entries.length, 1);
      expect(entries[0]['key'], 'locale');

      engine.close();
    });

    test('preferences persist across reopen', () {
      final path = '${tempDir.path}/persist';
      Directory(path).createSync();

      var engine = NoSqlEngine.open(bindings, path);
      engine.prefSet('app_prefs', 'theme', 'dark');
      engine.close();

      engine = NoSqlEngine.open(bindings, path);
      final resp = engine.prefGet('app_prefs', 'theme');
      expect(resp['found'], isTrue);
      expect(resp['value'], 'dark');
      engine.close();
    });

    test('separate stores are isolated', () {
      final engine = NoSqlEngine.open(bindings, tempDir.path);

      engine.prefSet('store_a', 'key', 'value_a');
      engine.prefSet('store_b', 'key', 'value_b');

      expect(engine.prefGet('store_a', 'key')['value'], 'value_a');
      expect(engine.prefGet('store_b', 'key')['value'], 'value_b');

      final keysA = engine.prefKeys('store_a');
      final keysB = engine.prefKeys('store_b');
      expect(keysA, ['key']);
      expect(keysB, ['key']);

      engine.close();
    });
  });

  group('NodeDB facade — Preferences', () {
    test('delegates preference operations', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      db.prefSet('prefs', 'locale', 'en');

      final resp = db.prefGet('prefs', 'locale');
      expect(resp['value'], 'en');

      final keys = db.prefKeys('prefs');
      expect(keys, ['locale']);

      expect(db.prefRemove('prefs', 'locale'), isTrue);

      final afterRemove = db.prefGet('prefs', 'locale');
      expect(afterRemove['found'], isFalse);

      db.close();
    });
  });
}
