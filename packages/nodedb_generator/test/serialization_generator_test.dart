import 'package:nodedb_generator/src/serialization_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateSerialization', () {
    test('generates fromMap function', () {
      final output = generateSerialization(userModel);
      expect(output, contains('User _\$UserFromMap(Map<String, dynamic> map)'));
      expect(output, contains('return User('));
    });

    test('generates toMap function', () {
      final output = generateSerialization(userModel);
      expect(output, contains('Map<String, dynamic> _\$UserToMap(User instance)'));
      expect(output, contains("'name': instance.name"));
      expect(output, contains("'email': instance.email"));
    });

    test('int fields use num cast', () {
      final output = generateSerialization(userModel);
      expect(output, contains("(map['id'] as num).toInt()"));
      expect(output, contains("(map['age'] as num).toInt()"));
    });

    test('double fields use num cast', () {
      final output = generateSerialization(articleModel);
      expect(output, contains("(map['rating'] as num).toDouble()"));
    });

    test('nullable DateTime uses conditional parse', () {
      final output = generateSerialization(userModel);
      expect(output, contains("map['createdAt'] != null ? DateTime.parse(map['createdAt'] as String) : null"));
    });

    test('non-nullable DateTime uses direct parse', () {
      final output = generateSerialization(knowsEdge);
      expect(output, contains("DateTime.parse(map['since'] as String)"));
    });

    test('DateTime serialization uses toIso8601String', () {
      final output = generateSerialization(knowsEdge);
      expect(output, contains('instance.since.toIso8601String()'));
    });

    test('nullable DateTime serialization uses ?. operator', () {
      final output = generateSerialization(userModel);
      expect(output, contains('instance.createdAt?.toIso8601String()'));
    });

    test('list fields cast to element type', () {
      final output = generateSerialization(articleModel);
      expect(output, contains("(map['tags'] as List? ?? []).cast<String>()"));
    });

    test('bool fields pass through', () {
      final output = generateSerialization(articleModel);
      expect(output, contains("map['draft'] as bool"));
    });

    test('String fields pass through', () {
      final output = generateSerialization(userModel);
      expect(output, contains("map['name'] as String"));
    });
  });

  group('generateSerialization — @JsonModel', () {
    test('standalone JsonModel generates fromMap/toMap', () {
      final output = generateSerialization(jsonModelFixture);
      expect(output, contains('ProductMetadata _\$ProductMetadataFromMap(Map<String, dynamic> map)'));
      expect(output, contains('Map<String, dynamic> _\$ProductMetadataToMap(ProductMetadata instance)'));
    });

    test('JsonModel field deserializes via _\$XFromMap', () {
      final output = generateSerialization(productWithJsonModel);
      expect(output, contains("_\$ProductMetadataFromMap(Map<String, dynamic>.from(map['metadata'] as Map? ?? {}))"));
    });

    test('JsonModel field serializes via _\$XToMap', () {
      final output = generateSerialization(productWithJsonModel);
      expect(output, contains('_\$ProductMetadataToMap(instance.metadata)'));
    });

    test('nullable JsonModel deserializes with null check', () {
      final output = generateSerialization(productWithNullableJsonModel);
      expect(output, contains("map['metadata'] != null ? _\$ProductMetadataFromMap(Map<String, dynamic>.from(map['metadata'] as Map)) : null"));
    });

    test('nullable JsonModel serializes with null check', () {
      final output = generateSerialization(productWithNullableJsonModel);
      expect(output, contains('instance.metadata != null ? _\$ProductMetadataToMap(instance.metadata!) : null'));
    });

    test('nested JsonModel deserializes inner model via _\$XFromMap', () {
      final output = generateSerialization(nestedJsonModelFixture);
      expect(output, contains("_\$ProductMetadataFromMap(Map<String, dynamic>.from(map['metadata'] as Map? ?? {}))"));
    });

    test('nested JsonModel serializes inner model via _\$XToMap', () {
      final output = generateSerialization(nestedJsonModelFixture);
      expect(output, contains('_\$ProductMetadataToMap(instance.metadata)'));
    });

    test('nested nullable JsonModel deserializes with null check', () {
      final output = generateSerialization(nestedJsonModelFixture);
      expect(output, contains("map['extra'] != null ? _\$ExtraInfoFromMap(Map<String, dynamic>.from(map['extra'] as Map)) : null"));
    });

    test('nested nullable JsonModel serializes with null check', () {
      final output = generateSerialization(nestedJsonModelFixture);
      expect(output, contains('instance.extra != null ? _\$ExtraInfoToMap(instance.extra!) : null'));
    });
  });
}
