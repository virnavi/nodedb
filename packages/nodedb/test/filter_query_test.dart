import 'package:nodedb/src/query/filter_query.dart';
import 'package:test/test.dart';

void main() {
  group('FilterQuery', () {
    test('equalTo builds single condition', () {
      final q = FilterQuery().equalTo('name', 'Alice');
      final result = q.build();
      expect(result['filter'], {
        'Condition': {'EqualTo': {'field': 'name', 'value': 'Alice'}},
      });
    });

    test('multiple conditions AND together', () {
      final q = FilterQuery()
          .equalTo('name', 'Alice')
          .greaterThan('age', 25);
      final result = q.build();
      final filter = result['filter'] as Map;
      expect(filter.containsKey('And'), isTrue);
      expect((filter['And'] as List).length, 2);
    });

    test('sortBy adds sort entries', () {
      final q = FilterQuery().sortBy('name').sortBy('age', desc: true);
      final result = q.build();
      expect(result['sort'], [
        {'field': 'name', 'direction': 'Asc'},
        {'field': 'age', 'direction': 'Desc'},
      ]);
    });

    test('offset and limit', () {
      final q = FilterQuery().offset(10).limit(5);
      final result = q.build();
      expect(result['offset'], 10);
      expect(result['limit'], 5);
    });

    test('or creates OR groups', () {
      final q = FilterQuery()
          .equalTo('role', 'admin')
          .or((q) => q.equalTo('role', 'superadmin'));
      final result = q.build();
      final filter = result['filter'] as Map;
      expect(filter.containsKey('Or'), isTrue);
      expect((filter['Or'] as List).length, 2);
    });

    test('between builds condition with low and high', () {
      final q = FilterQuery().between('age', 18, 65);
      final result = q.build();
      expect(result['filter'], {
        'Condition': {
          'Between': {'field': 'age', 'low': 18, 'high': 65},
        },
      });
    });

    test('inList with single value becomes equalTo', () {
      final q = FilterQuery().inList('status', ['active']);
      final result = q.build();
      expect(result['filter'], {
        'Condition': {'EqualTo': {'field': 'status', 'value': 'active'}},
      });
    });

    test('inList with multiple values creates OR groups', () {
      final q = FilterQuery().inList('status', ['active', 'pending', 'review']);
      final result = q.build();
      final filter = result['filter'] as Map;
      expect(filter.containsKey('Or'), isTrue);
      final orGroups = filter['Or'] as List;
      expect(orGroups.length, 3);
      // Each group is a single Condition with EqualTo
      for (final group in orGroups) {
        expect((group as Map).containsKey('Condition'), isTrue);
        expect((group['Condition'] as Map).containsKey('EqualTo'), isTrue);
      }
    });

    test('inList with empty list is no-op', () {
      final q = FilterQuery().inList('status', []);
      final result = q.build();
      expect(result['filter'], isNull);
    });

    test('notInList creates AND of NotEqualTo', () {
      final q = FilterQuery().notInList('status', ['deleted', 'archived']);
      final result = q.build();
      final filter = result['filter'] as Map;
      expect(filter.containsKey('And'), isTrue);
      final andGroups = filter['And'] as List;
      expect(andGroups.length, 2);
      for (final group in andGroups) {
        expect((group as Map)['Condition'], isA<Map>());
        expect(((group)['Condition'] as Map).containsKey('NotEqualTo'), isTrue);
      }
    });

    test('notInList with single value', () {
      final q = FilterQuery().notInList('status', ['deleted']);
      final result = q.build();
      expect(result['filter'], {
        'Condition': {'NotEqualTo': {'field': 'status', 'value': 'deleted'}},
      });
    });

    test('inList preserves existing conditions in OR', () {
      final q = FilterQuery()
          .greaterThan('age', 18)
          .inList('role', ['admin', 'editor']);
      final result = q.build();
      final filter = result['filter'] as Map;
      // Should have OR groups: [existing conditions, role=admin, role=editor]
      expect(filter.containsKey('Or'), isTrue);
      final orGroups = filter['Or'] as List;
      expect(orGroups.length, 3);
    });

    test('isNull/isNotNull build correctly', () {
      final q = FilterQuery().isNull('deletedAt');
      final result = q.build();
      expect(result['filter'], {
        'Condition': {'IsNull': {'field': 'deletedAt'}},
      });
    });

    test('empty query builds empty map', () {
      final result = FilterQuery().build();
      expect(result, isEmpty);
    });
  });

  group('FilterQuery — describe()', () {
    test('empty query describes as all records', () {
      expect(FilterQuery().describe(), 'all records');
    });

    test('single condition', () {
      final desc = FilterQuery().equalTo('name', 'Alice').describe();
      expect(desc, contains('name = Alice'));
    });

    test('multiple AND conditions', () {
      final desc = FilterQuery()
          .equalTo('name', 'Alice')
          .greaterThan('age', 25)
          .describe();
      expect(desc, contains('name = Alice'));
      expect(desc, contains('age > 25'));
      expect(desc, contains('AND'));
    });

    test('OR conditions', () {
      final desc = FilterQuery()
          .equalTo('role', 'admin')
          .or((q) => q.equalTo('role', 'super'))
          .describe();
      expect(desc, contains('OR'));
    });

    test('sort description', () {
      final desc = FilterQuery()
          .sortBy('name')
          .sortBy('age', desc: true)
          .describe();
      expect(desc, contains('sorted by name ascending'));
      expect(desc, contains('sorted by age descending'));
    });

    test('pagination description', () {
      final desc = FilterQuery().offset(10).limit(5).describe();
      expect(desc, contains('offset 10'));
      expect(desc, contains('limit 5'));
    });

    test('distinct flag description', () {
      final desc = FilterQuery().distinct().describe();
      expect(desc, contains('distinct'));
    });

    test('between condition', () {
      final desc = FilterQuery().between('age', 18, 65).describe();
      expect(desc, contains('age between 18 and 65'));
    });

    test('contains condition', () {
      final desc = FilterQuery().contains('name', 'Ali').describe();
      expect(desc, contains('name contains "Ali"'));
    });

    test('startsWith condition', () {
      final desc = FilterQuery().startsWith('email', 'admin').describe();
      expect(desc, contains('email starts with "admin"'));
    });

    test('endsWith condition', () {
      final desc = FilterQuery().endsWith('email', '.com').describe();
      expect(desc, contains('email ends with ".com"'));
    });

    test('isNull condition', () {
      final desc = FilterQuery().isNull('deletedAt').describe();
      expect(desc, contains('deletedAt is null'));
    });

    test('isNotNull condition', () {
      final desc = FilterQuery().isNotNull('email').describe();
      expect(desc, contains('email is not null'));
    });

    test('notEqualTo condition', () {
      final desc = FilterQuery().notEqualTo('status', 'deleted').describe();
      expect(desc, contains('status != deleted'));
    });

    test('lessThan condition', () {
      final desc = FilterQuery().lessThan('age', 18).describe();
      expect(desc, contains('age < 18'));
    });

    test('greaterThanOrEqual condition', () {
      final desc = FilterQuery().greaterThanOrEqual('score', 90).describe();
      expect(desc, contains('score >= 90'));
    });

    test('lessThanOrEqual condition', () {
      final desc = FilterQuery().lessThanOrEqual('price', 100).describe();
      expect(desc, contains('price <= 100'));
    });

    test('combined filter, sort, pagination, and distinct', () {
      final desc = FilterQuery()
          .equalTo('active', true)
          .sortBy('createdAt', desc: true)
          .offset(0)
          .limit(10)
          .distinct()
          .describe();
      expect(desc, contains('active = true'));
      expect(desc, contains('sorted by createdAt descending'));
      expect(desc, contains('offset 0'));
      expect(desc, contains('limit 10'));
      expect(desc, contains('distinct'));
    });
  });
}
