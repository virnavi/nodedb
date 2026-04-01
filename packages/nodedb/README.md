# nodedb

Embedded multi-engine database for Flutter. Provides typed wrappers around NoSQL, Graph, Vector, Federation, DAC, Provenance, Key Resolver, Schema, Transport, and AI engines — all accessible through a single `NodeDB` facade.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dependencies:
  nodedb:
    path: ../nodedb
```

## Usage

```dart
import 'package:nodedb/nodedb.dart';

final db = NodeDB.open(
  directory: '/path/to/db',
  databaseName: 'main',
);

// NoSQL CRUD
db.nosql.put('users', {'name': 'Alice', 'email': 'alice@example.com'});
final doc = db.nosql.get('users', id);

// Typed DAOs via code generation
final users = UserDao(db.nosql);
users.create(User(name: 'Alice', email: 'alice@example.com'));
final alice = users.findFirst((q) => q.nameEqualTo('Alice'));
```

## Annotations

Define models with annotations, then run `build_runner` to generate schemas, serialization, typed query builders, and DAOs:

- `@collection` — NoSQL collection with String UUID id
- `@node` / `@Edge` — Graph nodes and edges
- `@preferences` — Encrypted per-key preference store
- `@Collection(singleton: true)` — Single-record collections
- `@Index`, `@Trimmable`, `@neverTrim`, `@Trigger`, `@Shareable`
