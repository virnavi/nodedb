import 'package:nodedb_generator/src/preferences_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generatePreferencesDao', () {
    late String output;

    setUp(() {
      output = generatePreferencesDao(appPrefsModel);
    });

    test('generates prefs base class', () {
      expect(output, contains('abstract class AppPrefsPrefsBase'));
    });

    test('has correct store name', () {
      expect(output, contains("String get storeName => 'app_prefs'"));
    });

    test('generates getter for each data field', () {
      expect(output, contains('getLocale()'));
      expect(output, contains('getFontSize()'));
      expect(output, contains('getDarkMode()'));
    });

    test('generates setter for each data field', () {
      expect(output, contains('setLocale(String value'));
      expect(output, contains('setFontSize(int value'));
      expect(output, contains('setDarkMode(bool value'));
    });

    test('generates remover for each data field', () {
      expect(output, contains('removeLocale()'));
      expect(output, contains('removeFontSize()'));
      expect(output, contains('removeDarkMode()'));
    });

    test('excludes id field from accessors', () {
      expect(output, isNot(contains('getId()')));
      expect(output, isNot(contains('setId(')));
      expect(output, isNot(contains('removeId()')));
    });

    test('generates allKeys method', () {
      expect(output, contains('List<String> allKeys()'));
      expect(output, contains('_engine.prefKeys(storeName)'));
    });

    test('generates shareableEntries method', () {
      expect(output, contains('shareableEntries()'));
      expect(output, contains('_engine.prefShareable(storeName)'));
    });

    test('setters have shareable parameter', () {
      expect(output, contains('{bool shareable = false}'));
    });

    test('getters delegate to prefGet', () {
      expect(output, contains("_engine.prefGet(storeName, 'locale')"));
      expect(output, contains("_engine.prefGet(storeName, 'fontSize')"));
      expect(output, contains("_engine.prefGet(storeName, 'darkMode')"));
    });

    test('setters delegate to prefSet', () {
      expect(output, contains("_engine.prefSet(storeName, 'locale'"));
      expect(output, contains("_engine.prefSet(storeName, 'fontSize'"));
      expect(output, contains("_engine.prefSet(storeName, 'darkMode'"));
    });

    test('removers delegate to prefRemove', () {
      expect(output, contains("_engine.prefRemove(storeName, 'locale')"));
      expect(output, contains("_engine.prefRemove(storeName, 'fontSize')"));
      expect(output, contains("_engine.prefRemove(storeName, 'darkMode')"));
    });

    test('uses NoSqlEngine', () {
      expect(output, contains('NoSqlEngine get _engine'));
    });

    test('has preferences comment header', () {
      expect(output, contains('Preferences Accessor'));
    });

    test('getters return nullable type for non-nullable fields', () {
      // Non-nullable fields should still return nullable from prefs (key may not exist)
      expect(output, contains('String? getLocale()'));
      expect(output, contains('int? getFontSize()'));
      expect(output, contains('bool? getDarkMode()'));
    });

    test('removers return bool', () {
      expect(output, contains('bool removeLocale()'));
      expect(output, contains('bool removeFontSize()'));
      expect(output, contains('bool removeDarkMode()'));
    });

    test('generates removeAll method', () {
      expect(output, contains('void removeAll()'));
    });

    test('removeAll iterates allKeys', () {
      expect(output, contains('for (final key in allKeys())'));
      expect(output, contains('_engine.prefRemove(storeName, key)'));
    });

    test('has balanced braces', () {
      final openBraces = '{'.allMatches(output).length;
      final closeBraces = '}'.allMatches(output).length;
      expect(openBraces, closeBraces);
    });
  });
}
