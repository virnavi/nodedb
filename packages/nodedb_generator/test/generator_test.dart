import 'package:nodedb_generator/src/schema_generator.dart';
import 'package:nodedb_generator/src/serialization_generator.dart';
import 'package:nodedb_generator/src/filter_generator.dart';
import 'package:nodedb_generator/src/dao_generator.dart';
import 'package:nodedb_generator/src/dao_registry_generator.dart';
import 'package:nodedb_generator/src/preferences_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('Full pipeline — collection', () {
    late String schema;
    late String serialization;
    late String filters;
    late String dao;
    late String concreteDao;

    setUp(() {
      schema = generateSchema(userModel);
      serialization = generateSerialization(userModel);
      filters = generateFilterExtensions(userModel);
      dao = generateDao(userModel);
      concreteDao = generateConcreteDao(userModel);
    });

    test('all sections generate non-empty output', () {
      expect(schema, isNotEmpty);
      expect(serialization, isNotEmpty);
      expect(filters, isNotEmpty);
      expect(dao, isNotEmpty);
      expect(concreteDao, isNotEmpty);
    });

    test('combined output has balanced braces', () {
      final combined = '$schema\n$serialization\n$filters\n$dao\n$concreteDao';
      final openBraces = '{'.allMatches(combined).length;
      final closeBraces = '}'.allMatches(combined).length;
      expect(openBraces, closeBraces);
    });

    test('combined output has balanced parentheses', () {
      final combined = '$schema\n$serialization\n$filters\n$dao\n$concreteDao';
      final openParens = '('.allMatches(combined).length;
      final closeParens = ')'.allMatches(combined).length;
      expect(openParens, closeParens);
    });

    test('cross-references are consistent', () {
      // DAO references serialization functions
      expect(dao, contains('_\$UserFromMap'));
      expect(serialization, contains('User _\$UserFromMap'));
      // Concrete DAO extends base DAO
      expect(dao, contains('abstract class UserDaoBase'));
      expect(concreteDao, contains('class UserDao extends UserDaoBase'));
    });
  });

  group('Full pipeline — node', () {
    test('all sections generate for node model', () {
      final schema = generateSchema(personNode);
      final serialization = generateSerialization(personNode);
      final filters = generateFilterExtensions(personNode);
      final dao = generateDao(personNode);
      final concreteDao = generateConcreteDao(personNode);

      expect(schema, contains("type: 'node'"));
      expect(serialization, contains('Person _\$PersonFromMap'));
      expect(filters, contains('PersonFilterExtension'));
      expect(dao, contains('PersonNodeDaoBase'));
      expect(concreteDao, contains('PersonDao extends PersonNodeDaoBase'));
    });
  });

  group('Full pipeline — edge', () {
    test('all sections generate for edge model', () {
      final schema = generateSchema(knowsEdge);
      final serialization = generateSerialization(knowsEdge);
      final filters = generateFilterExtensions(knowsEdge);
      final dao = generateDao(knowsEdge);
      final concreteDao = generateConcreteDao(knowsEdge);

      expect(schema, contains("type: 'edge'"));
      expect(serialization, contains('Knows _\$KnowsFromMap'));
      expect(filters, contains('KnowsFilterExtension'));
      expect(dao, contains('KnowsEdgeDaoBase'));
      expect(concreteDao, contains('KnowsDao extends KnowsEdgeDaoBase'));
    });
  });

  group('Full pipeline — noDao', () {
    test('skips DAO generation when generateDao is false', () {
      // The main generator checks model.generateDao before calling generateDao/generateConcreteDao
      // Here we verify the model flag is correctly set
      expect(noDaoModel.generateDao, isFalse);
    });
  });

  group('Full pipeline — singleton', () {
    late String schema;
    late String serialization;
    late String filters;
    late String dao;
    late String concreteDao;

    setUp(() {
      schema = generateSchema(settingsModel);
      serialization = generateSerialization(settingsModel);
      filters = generateFilterExtensions(settingsModel);
      dao = generateDao(settingsModel);
      concreteDao = generateConcreteDao(settingsModel);
    });

    test('all sections generate non-empty output', () {
      expect(schema, isNotEmpty);
      expect(serialization, isNotEmpty);
      expect(filters, isNotEmpty);
      expect(dao, isNotEmpty);
      expect(concreteDao, isNotEmpty);
    });

    test('schema shows singleton: true', () {
      expect(schema, contains('singleton: true'));
    });

    test('DAO uses singleton pattern instead of CRUD', () {
      expect(dao, contains('init('));
      expect(dao, contains('reset()'));
      expect(dao, isNot(contains('findById')));
      expect(dao, isNot(contains('deleteById')));
    });

    test('combined output has balanced braces', () {
      final combined = '$schema\n$serialization\n$filters\n$dao\n$concreteDao';
      final openBraces = '{'.allMatches(combined).length;
      final closeBraces = '}'.allMatches(combined).length;
      expect(openBraces, closeBraces);
    });

    test('cross-references are consistent', () {
      expect(dao, contains('_\$SettingsFromMap'));
      expect(serialization, contains('Settings _\$SettingsFromMap'));
      expect(dao, contains('abstract class SettingsDaoBase'));
      expect(concreteDao, contains('SettingsDao extends SettingsDaoBase'));
    });
  });

  group('Full pipeline — preferences', () {
    late String serialization;
    late String prefsDao;

    setUp(() {
      serialization = generateSerialization(appPrefsModel);
      prefsDao = generatePreferencesDao(appPrefsModel);
    });

    test('generates serialization and prefs accessor', () {
      expect(serialization, isNotEmpty);
      expect(prefsDao, isNotEmpty);
    });

    test('preferences does NOT generate schema or filters', () {
      // Preferences models skip schema and filter generation in the main generator
      // They only produce serialization + typed accessor
      expect(prefsDao, isNot(contains('NodeDbSchema')));
      expect(prefsDao, isNot(contains('FilterExtension')));
    });

    test('prefs accessor has per-field methods', () {
      expect(prefsDao, contains('getLocale'));
      expect(prefsDao, contains('setLocale'));
      expect(prefsDao, contains('removeLocale'));
      expect(prefsDao, contains('getFontSize'));
      expect(prefsDao, contains('setFontSize'));
      expect(prefsDao, contains('removeFontSize'));
      expect(prefsDao, contains('getDarkMode'));
      expect(prefsDao, contains('setDarkMode'));
      expect(prefsDao, contains('removeDarkMode'));
    });

    test('combined output has balanced braces', () {
      final combined = '$serialization\n$prefsDao';
      final openBraces = '{'.allMatches(combined).length;
      final closeBraces = '}'.allMatches(combined).length;
      expect(openBraces, closeBraces);
    });
  });

  group('DAO registry with multiple models', () {
    test('registry includes all models', () {
      final registry = generateDaoRegistry([userModel, articleModel, personNode]);
      expect(registry, contains('final UserDao users'));
      expect(registry, contains('final ArticleDao articles'));
      expect(registry, contains('final PersonDao people'));
    });
  });
}
