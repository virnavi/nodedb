import 'package:nodedb_generator/src/view_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateViewDao', () {
    late String output;

    setUp(() {
      output = generateViewDao(userProductView);
    });

    test('generates abstract ViewDaoBase class', () {
      expect(output, contains('abstract class UserProductViewViewDaoBase'));
    });

    test('has NoSqlEngine getter', () {
      expect(output, contains('NoSqlEngine get _engine'));
    });

    test('has optional TransportEngine getter', () {
      expect(output, contains('TransportEngine? get _transport'));
    });

    test('has optional databaseName getter', () {
      expect(output, contains('String? get _databaseName'));
    });

    test('has schemaName constant', () {
      expect(output, contains("static const schemaName = 'public'"));
    });

    test('has qualifiedName getter', () {
      expect(output, contains('String get qualifiedName'));
    });

    test('has collectionName getter', () {
      expect(output, contains("String get collectionName => 'user_product_views'"));
    });

    test('has static sources list', () {
      expect(output, contains('static const sources'));
      expect(output, contains("'collection': 'users'"));
      expect(output, contains("'collection': 'products'"));
      expect(output, contains("'database': 'warehouse'"));
    });

    test('generates findAll method', () {
      expect(output, contains('List<UserProductView> findAll'));
    });

    test('generates findFirst method', () {
      expect(output, contains('UserProductView? findFirst'));
    });

    test('generates findWhere method', () {
      expect(output, contains('List<UserProductView> findWhere'));
    });

    test('generates count method', () {
      expect(output, contains('int count()'));
    });

    test('does NOT generate create/save/delete methods', () {
      expect(output, isNot(contains('void create(')));
      expect(output, isNot(contains('void save(')));
      expect(output, isNot(contains('deleteById(')));
      expect(output, isNot(contains('void createAll(')));
    });

    test('queries remote sources via transport', () {
      expect(output, contains('_transport!.meshQuery'));
    });

    test('queries local sources via engine', () {
      expect(output, contains('_engine.findAll'));
    });
  });

  group('generateConcreteViewDao', () {
    late String output;

    setUp(() {
      output = generateConcreteViewDao(userProductView);
    });

    test('generates concrete class extending base', () {
      expect(output, contains('class UserProductViewDao extends UserProductViewViewDaoBase'));
    });

    test('has NoSqlEngine field', () {
      expect(output, contains('final NoSqlEngine _engine'));
    });

    test('has optional TransportEngine field', () {
      expect(output, contains('final TransportEngine? _transport'));
    });

    test('has optional databaseName field', () {
      expect(output, contains('final String? _databaseName'));
    });
  });
}
