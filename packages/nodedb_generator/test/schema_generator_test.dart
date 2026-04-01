import 'package:nodedb_generator/src/model_info.dart';
import 'package:nodedb_generator/src/schema_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateSchema', () {
    test('generates schema constant for collection', () {
      final output = generateSchema(userModel);
      expect(output, contains('const _userSchema = NodeDbSchema('));
      expect(output, contains("name: 'users'"));
      expect(output, contains("schema: 'public'"));
      expect(output, contains('singleton: false'));
      expect(output, contains("type: 'collection'"));
    });

    test('includes all fields', () {
      final output = generateSchema(userModel);
      expect(output, contains("SchemaField('id',"));
      expect(output, contains("SchemaField('name',"));
      expect(output, contains("SchemaField('email',"));
      expect(output, contains("SchemaField('age',"));
      expect(output, contains("SchemaField('createdAt',"));
    });

    test('marks indexed fields', () {
      final output = generateSchema(userModel);
      // email has index with unique: true
      expect(output, contains("SchemaField('email', 'string', indexed: true, unique: true)"));
    });

    test('maps field types correctly', () {
      final output = generateSchema(articleModel);
      expect(output, contains("SchemaField('id', 'int'"));
      expect(output, contains("SchemaField('title', 'string'"));
      expect(output, contains("SchemaField('rating', 'double'"));
      expect(output, contains("SchemaField('draft', 'bool'"));
      expect(output, contains("SchemaField('publishedAt', 'datetime'"));
      expect(output, contains("SchemaField('tags', 'list'"));
    });

    test('handles singleton flag', () {
      final output = generateSchema(settingsModel);
      expect(output, contains('singleton: true'));
    });

    test('generates node schema', () {
      final output = generateSchema(personNode);
      expect(output, contains("type: 'node'"));
      expect(output, contains("name: 'people'"));
    });

    test('generates edge schema', () {
      final output = generateSchema(knowsEdge);
      expect(output, contains("type: 'edge'"));
      expect(output, contains("name: 'knows'"));
    });
  });

  group('generateSchema — String id auto-index', () {
    test('String id field auto-marked as indexed+unique', () {
      final output = generateSchema(userModelStringId);
      expect(output, contains("SchemaField('id', 'string', indexed: true, unique: true)"));
    });

    test('int id field NOT auto-marked as indexed', () {
      final output = generateSchema(userModel);
      expect(output, contains("SchemaField('id', 'int')"));
      // Should not have indexed: true for int id (no explicit index annotation)
      expect(output, isNot(contains("SchemaField('id', 'int', indexed: true")));
    });
  });

  group('generateSchema — trim fields', () {
    test('trimmable model includes trimmable flag', () {
      final output = generateSchema(trimmableModel);
      expect(output, contains('trimmable: true'));
    });

    test('trimmable model includes trim policy', () {
      final output = generateSchema(trimmableModel);
      expect(output, contains("trimPolicy: 'age:30d'"));
    });

    test('neverTrim model includes neverTrim flag', () {
      final output = generateSchema(neverTrimModel);
      expect(output, contains('neverTrim: true'));
    });

    test('regular model does not include trim flags', () {
      final output = generateSchema(userModel);
      expect(output, isNot(contains('trimmable: true')));
      expect(output, isNot(contains('neverTrim: true')));
    });
  });

  group('generateSchema — JSON export', () {
    test('generates schema JSON constant', () {
      final output = generateSchema(userModel);
      expect(output, contains('const _userSchemaJson'));
    });

    test('JSON export includes collection name', () {
      final output = generateSchema(userModel);
      expect(output, contains("'name': 'users'"));
    });

    test('JSON export includes schema', () {
      final output = generateSchema(userModel);
      expect(output, contains("'schema': 'public'"));
    });

    test('JSON export includes type', () {
      final output = generateSchema(userModel);
      expect(output, contains("'type': 'collection'"));
    });

    test('JSON export includes required_fields (non-nullable, non-id)', () {
      final output = generateSchema(userModel);
      expect(output, contains("'required_fields'"));
      expect(output, contains("'name'"));
      expect(output, contains("'email'"));
      expect(output, contains("'age'"));
    });

    test('JSON export excludes nullable fields from required', () {
      final output = generateSchema(userModel);
      // createdAt is nullable, should NOT be in required_fields
      // But it will be in field_types — check required_fields block only
      final requiredBlock = output.substring(
        output.indexOf("'required_fields'"),
        output.indexOf("'field_types'"),
      );
      expect(requiredBlock, isNot(contains("'createdAt'")));
    });

    test('JSON export includes field_types map', () {
      final output = generateSchema(articleModel);
      expect(output, contains("'field_types'"));
      expect(output, contains("'title': 'string'"));
      expect(output, contains("'rating': 'double'"));
      expect(output, contains("'draft': 'bool'"));
      expect(output, contains("'tags': 'list'"));
    });

    test('node schema JSON export', () {
      final output = generateSchema(personNode);
      expect(output, contains('const _personSchemaJson'));
      expect(output, contains("'type': 'node'"));
    });
  });

  group('generateSchemaTypes', () {
    test('produces NodeDbSchema class', () {
      final output = generateSchemaTypes();
      expect(output, contains('class NodeDbSchema'));
      expect(output, contains('final String name'));
      expect(output, contains('final List<SchemaField> fields'));
    });

    test('produces SchemaField class', () {
      final output = generateSchemaTypes();
      expect(output, contains('class SchemaField'));
      expect(output, contains('final bool indexed'));
      expect(output, contains('final bool unique'));
    });

    test('NodeDbSchema includes trim fields', () {
      final output = generateSchemaTypes();
      expect(output, contains('final bool trimmable'));
      expect(output, contains('final String? trimPolicy'));
      expect(output, contains('final bool neverTrim'));
    });

    test('NodeDbSchema has qualifiedName getter', () {
      final output = generateSchemaTypes();
      expect(output, contains('String get qualifiedName'));
    });
  });

  group('generateSchema — FQN constant', () {
    test('generates collection name constant', () {
      final output = generateSchema(userModel);
      expect(output, contains("const userCollectionName = 'public.users'"));
    });

    test('generates constant for article model', () {
      final output = generateSchema(articleModel);
      expect(output, contains("const articleCollectionName = 'public.articles'"));
    });
  });

  group('generateSchema — JSONB field type', () {
    test('JSONB field type is jsonb', () {
      final output = generateSchema(productModelWithJsonb);
      expect(output, contains("'jsonb'"));
    });

    test('JsonModel field type is jsonb', () {
      final output = generateSchema(productWithJsonModel);
      expect(output, contains("SchemaField('metadata', 'jsonb')"));
    });
  });

  group('generateSchema — isJsonbLike', () {
    test('isJsonbLike includes isJsonb fields', () {
      final field = FieldInfo(name: 'data', dartType: 'Map<String, dynamic>', isJsonb: true);
      expect(field.isJsonbLike, isTrue);
    });

    test('isJsonbLike includes isJsonModel fields', () {
      final field = FieldInfo(name: 'meta', dartType: 'ProductMetadata', isJsonModel: true, jsonModelType: 'ProductMetadata');
      expect(field.isJsonbLike, isTrue);
    });

    test('isJsonbLike is false for regular fields', () {
      final field = FieldInfo(name: 'name', dartType: 'String');
      expect(field.isJsonbLike, isFalse);
    });
  });
}
