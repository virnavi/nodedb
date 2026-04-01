# nodedb_test

Test utilities for NodeDB. Provides `TestNodeDB` helper for creating temporary database instances in tests, along with custom matchers and seed helpers.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dev_dependencies:
  nodedb_test:
    path: ../nodedb_test
```

## Usage

```dart
import 'package:nodedb_test/nodedb_test.dart';

void main() {
  late TestNodeDB testDb;

  setUp(() {
    testDb = TestNodeDB();
  });

  tearDown(() {
    testDb.dispose();
  });

  test('example', () {
    final db = testDb.db;
    db.nosql.put('users', {'name': 'Alice'});
    expect(db.nosql.count('users'), equals(1));
  });
}
```
