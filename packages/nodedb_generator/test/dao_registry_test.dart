import 'package:nodedb_generator/src/dao_registry_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateDaoRegistry', () {
    test('generates NodeDbDaos class', () {
      final output = generateDaoRegistry([userModel, articleModel]);
      expect(output, contains('class NodeDbDaos'));
    });

    test('has typed DAO fields', () {
      final output = generateDaoRegistry([userModel, articleModel]);
      expect(output, contains('final UserDao users'));
      expect(output, contains('final ArticleDao articles'));
    });

    test('constructor accepts NoSqlEngine for collections', () {
      final output = generateDaoRegistry([userModel]);
      expect(output, contains('NoSqlEngine nosqlEngine'));
      expect(output, contains('ProvenanceEngine? provenanceEngine'));
    });

    test('constructor accepts both engines for mixed models', () {
      final output = generateDaoRegistry([userModel, personNode]);
      expect(output, contains('NoSqlEngine nosqlEngine'));
      expect(output, contains('GraphEngine graphEngine'));
    });

    test('initializer list creates DAOs', () {
      final output = generateDaoRegistry([userModel, articleModel]);
      expect(output, contains('users = UserDao(nosqlEngine, provenanceEngine, notifier, databaseName)'));
      expect(output, contains('articles = ArticleDao(nosqlEngine, provenanceEngine, notifier, databaseName)'));
    });

    test('graph DAOs use graphEngine', () {
      final output = generateDaoRegistry([personNode]);
      expect(output, contains('people = PersonDao(graphEngine)'));
    });

    test('handles single model', () {
      final output = generateDaoRegistry([userModel]);
      expect(output, contains('final UserDao users'));
      expect(output, contains('users = UserDao(nosqlEngine, provenanceEngine, notifier, databaseName)'));
    });
  });

  group('generateDaoExtension', () {
    test('generates extension on NodeDB', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('extension NodeDbDaoAccess on NodeDB'));
    });

    test('generates dao getter returning NodeDbDaos', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('NodeDbDaos get dao'));
    });

    test('passes nosql engine for collection models', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('nosql'));
    });

    test('passes graph engine for node models', () {
      final output = generateDaoExtension([personNode]);
      expect(output, contains('graph!'));
    });

    test('passes both engines for mixed models', () {
      final output = generateDaoExtension([userModel, personNode]);
      expect(output, contains('nosql'));
      expect(output, contains('graph!'));
    });

    test('passes provenanceEngine parameter', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('provenanceEngine: provenance'));
    });

    test('passes databaseName from NodeDB', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('databaseName: databaseName'));
    });

    test('passes transport parameter', () {
      final output = generateDaoExtension([userModel]);
      expect(output, contains('transport: transport'));
    });
  });

  group('generateDaoRegistry — databaseName', () {
    test('constructor accepts databaseName param', () {
      final output = generateDaoRegistry([userModel]);
      expect(output, contains('String? databaseName'));
    });

    test('collection DAOs receive databaseName', () {
      final output = generateDaoRegistry([userModel]);
      expect(output, contains('databaseName)'));
    });
  });

  group('generateDaoRegistry — views', () {
    test('view DAO field is generated', () {
      final output = generateDaoRegistry([userProductView]);
      expect(output, contains('final UserProductViewDao user_product_views'));
    });

    test('view DAO receives transport in init', () {
      final output = generateDaoRegistry([userProductView]);
      expect(output, contains('UserProductViewDao(nosqlEngine, transport, databaseName)'));
    });

    test('constructor accepts TransportEngine for views', () {
      final output = generateDaoRegistry([userProductView]);
      expect(output, contains('TransportEngine? transport'));
    });
  });
}
