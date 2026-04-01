import 'package:nodedb_generator/src/filter_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateFilterExtensions', () {
    test('generates extension with correct name', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('extension UserFilterExtension on FilterQuery<User>'));
    });

    test('generates equalTo/notEqualTo for all fields', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('nameEqualTo(String value)'));
      expect(output, contains('nameNotEqualTo(String value)'));
      expect(output, contains('ageEqualTo(int value)'));
      expect(output, contains('ageNotEqualTo(int value)'));
    });

    test('generates string-specific filters', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('nameContains(String value)'));
      expect(output, contains('nameStartsWith(String value)'));
      expect(output, contains('nameEndsWith(String value)'));
      expect(output, contains('emailContains(String value)'));
    });

    test('generates numeric comparison operators', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('ageGreaterThan(int value)'));
      expect(output, contains('ageGreaterThanOrEqual(int value)'));
      expect(output, contains('ageLessThan(int value)'));
      expect(output, contains('ageLessThanOrEqual(int value)'));
      expect(output, contains('ageBetween(int low, int high)'));
    });

    test('generates DateTime comparison operators', () {
      final output = generateFilterExtensions(knowsEdge);
      expect(output, contains('sinceGreaterThan(DateTime value)'));
      expect(output, contains('sinceLessThan(DateTime value)'));
      expect(output, contains('sinceBetween(DateTime low, DateTime high)'));
    });

    test('generates isNull/isNotNull for nullable fields', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('createdAtIsNull()'));
      expect(output, contains('createdAtIsNotNull()'));
    });

    test('does not generate isNull for non-nullable fields', () {
      final output = generateFilterExtensions(userModel);
      expect(output, isNot(contains('nameIsNull()')));
      expect(output, isNot(contains('ageIsNull()')));
    });

    test('generates double comparison operators', () {
      final output = generateFilterExtensions(articleModel);
      expect(output, contains('ratingGreaterThan(double value)'));
      expect(output, contains('ratingBetween(double low, double high)'));
    });

    test('bool fields only get equalTo/notEqualTo', () {
      final output = generateFilterExtensions(articleModel);
      expect(output, contains('draftEqualTo(bool value)'));
      expect(output, contains('draftNotEqualTo(bool value)'));
      // No greaterThan/lessThan for bool
      expect(output, isNot(contains('draftGreaterThan')));
      expect(output, isNot(contains('draftContains')));
    });

    test('list fields generate array operators instead of scalar filters', () {
      final output = generateFilterExtensions(articleModel);
      // No scalar filters for list fields
      expect(output, isNot(contains('tagsEqualTo')));
      // But array operators are generated
      expect(output, contains('tagsContains'));
      expect(output, contains('tagsOverlaps'));
    });

    test('generates sortBy helpers', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('sortByName({bool desc = false})'));
      expect(output, contains('sortByAge({bool desc = false})'));
      expect(output, contains('sortByEmail({bool desc = false})'));
    });

    test('skips list fields in sortBy', () {
      final output = generateFilterExtensions(articleModel);
      expect(output, isNot(contains('sortByTags')));
    });

    test('delegates to FilterQuery methods', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains("equalTo('name', value)"));
      expect(output, contains("greaterThan('age', value)"));
      expect(output, contains("contains('name', value)"));
      expect(output, contains("sortBy('name', desc: desc)"));
    });

    test('generates inList/notInList for all filterable fields', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains('nameInList(List<String> values)'));
      expect(output, contains('nameNotInList(List<String> values)'));
      expect(output, contains('ageInList(List<int> values)'));
      expect(output, contains('ageNotInList(List<int> values)'));
    });

    test('inList/notInList delegates to FilterQuery methods', () {
      final output = generateFilterExtensions(userModel);
      expect(output, contains("inList('name', values)"));
      expect(output, contains("notInList('name', values)"));
    });

    test('skips inList for list and embedded fields', () {
      final output = generateFilterExtensions(articleModel);
      expect(output, isNot(contains('tagsInList')));
      expect(output, isNot(contains('tagsNotInList')));
    });

    test('JSONB fields generate path/key/contains operators', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, contains('metadataPathEquals(String path, dynamic value)'));
      expect(output, contains('metadataHasKey(String path)'));
      expect(output, contains('metadataContains(Map<String, dynamic> value)'));
    });

    test('JSONB fields do not get scalar filters', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, isNot(contains('metadataEqualTo')));
      expect(output, isNot(contains('metadataNotEqualTo')));
      expect(output, isNot(contains('metadataInList')));
    });

    test('JSONB fields delegate to FilterQuery jsonb methods', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, contains("jsonPathEquals('metadata', path, value)"));
      expect(output, contains("jsonHasKey('metadata', path)"));
      expect(output, contains("jsonContains('metadata', value)"));
    });

    test('JSONB fields are skipped in sortBy', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, isNot(contains('sortByMetadata')));
    });

    test('List fields generate arrayContains and arrayOverlap', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, contains('categoriesContains(String value)'));
      expect(output, contains('categoriesOverlaps(List<String> values)'));
    });

    test('List fields delegate to FilterQuery array methods', () {
      final output = generateFilterExtensions(productModelWithJsonb);
      expect(output, contains("arrayContains('categories', value)"));
      expect(output, contains("arrayOverlap('categories', values)"));
    });

    test('JsonModel fields get JSONB path operators', () {
      final output = generateFilterExtensions(productWithJsonModel);
      expect(output, contains('metadataPathEquals(String path, dynamic value)'));
      expect(output, contains('metadataHasKey(String path)'));
      expect(output, contains('metadataContains(Map<String, dynamic> value)'));
    });

    test('JsonModel fields do not get scalar filters', () {
      final output = generateFilterExtensions(productWithJsonModel);
      expect(output, isNot(contains('metadataEqualTo')));
      expect(output, isNot(contains('metadataNotEqualTo')));
    });

    test('JsonModel fields are skipped in sortBy', () {
      final output = generateFilterExtensions(productWithJsonModel);
      expect(output, isNot(contains('sortByMetadata')));
    });
  });
}
