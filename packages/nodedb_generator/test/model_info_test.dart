import 'package:nodedb_generator/src/model_info.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('toSnakeCase', () {
    test('PascalCase', () {
      expect(toSnakeCase('UserProfile'), 'user_profile');
    });

    test('simple name', () {
      expect(toSnakeCase('User'), 'user');
    });

    test('multiple capitals', () {
      expect(toSnakeCase('HTTPResponse'), 'h_t_t_p_response');
    });

    test('single word', () {
      expect(toSnakeCase('Article'), 'article');
    });
  });

  group('pluralize', () {
    test('regular word adds s', () {
      expect(pluralize('user'), 'users');
      expect(pluralize('article'), 'articles');
    });

    test('sh/ch/x adds es', () {
      expect(pluralize('brush'), 'brushes');
      expect(pluralize('match'), 'matches');
      expect(pluralize('box'), 'boxes');
    });

    test('consonant+y becomes ies', () {
      expect(pluralize('category'), 'categories');
      expect(pluralize('entry'), 'entries');
    });

    test('vowel+y adds s', () {
      expect(pluralize('key'), 'keys');
      expect(pluralize('day'), 'days');
      expect(pluralize('toy'), 'toys');
    });

    test('word ending in s adds es', () {
      expect(pluralize('bus'), 'buses');
    });
  });

  group('ModelInfo', () {
    test('dataFields excludes id', () {
      expect(userModel.dataFields.map((f) => f.name),
          ['name', 'email', 'age', 'createdAt']);
    });

    test('idField returns id field', () {
      expect(userModel.idField, isNotNull);
      expect(userModel.idField!.name, 'id');
    });

    test('idField returns null when no id', () {
      final model = ModelInfo(
        className: 'NoId',
        collectionName: 'no_ids',
        type: ModelType.collection,
        fields: [FieldInfo(name: 'name', dartType: 'String')],
        indexes: [],
      );
      expect(model.idField, isNull);
    });
  });

  group('ModelInfo — id type detection', () {
    test('isStringId true for String id', () {
      expect(userModelStringId.isStringId, isTrue);
      expect(userModelStringId.isIntId, isFalse);
    });

    test('isIntId true for int id', () {
      expect(userModel.isStringId, isFalse);
      expect(userModel.isIntId, isTrue);
    });

    test('isIntId true when no id field', () {
      final model = ModelInfo(
        className: 'NoId',
        collectionName: 'no_ids',
        type: ModelType.collection,
        fields: [FieldInfo(name: 'name', dartType: 'String')],
        indexes: [],
      );
      expect(model.isIntId, isTrue);
      expect(model.isStringId, isFalse);
    });

    test('node model has int id', () {
      expect(personNode.isIntId, isTrue);
      expect(personNode.isStringId, isFalse);
    });
  });

  group('FieldInfo', () {
    test('isNumeric for int/double/num', () {
      expect(FieldInfo(name: 'x', dartType: 'int').isNumeric, isTrue);
      expect(FieldInfo(name: 'x', dartType: 'double').isNumeric, isTrue);
      expect(FieldInfo(name: 'x', dartType: 'num').isNumeric, isTrue);
      expect(FieldInfo(name: 'x', dartType: 'String').isNumeric, isFalse);
    });

    test('isString', () {
      expect(FieldInfo(name: 'x', dartType: 'String').isString, isTrue);
      expect(FieldInfo(name: 'x', dartType: 'int').isString, isFalse);
    });

    test('isBool', () {
      expect(FieldInfo(name: 'x', dartType: 'bool').isBool, isTrue);
    });

    test('isDateTime', () {
      expect(FieldInfo(name: 'x', dartType: 'DateTime').isDateTime, isTrue);
    });
  });
}
