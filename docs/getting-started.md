# Getting Started

[← Back to Index](README.md)

## Prerequisites

- **Rust** (stable, edition 2021) — for building the native library
- **Dart SDK** (≥3.0) — for the Dart/Flutter packages
- **Flutter** (≥3.0) — for the example app and Flutter integration

## Building the Native Library

```bash
cd rust/
cargo build --release -p nodedb-ffi
```

This produces `target/release/libnodedb_ffi.dylib` (macOS), `.so` (Linux), or `.dll` (Windows).

## Running Tests

```bash
# All Rust tests
cd rust/
cargo test

# Specific crate
cargo test -p nodedb-nosql
cargo test -p nodedb-transport
cargo test -p nodedb-ffi

# Dart tests
cd packages/nodedb/
dart test

cd packages/nodedb_generator/
dart test
```

## Basic Usage (Dart)

### 1. Open a Database

```dart
import 'package:nodedb/nodedb.dart';

final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'main',
  provenanceEnabled: true,      // optional
);
```

### 2. Write Documents

```dart
// Single write
db.writeTxn([
  WriteOp.put('users', data: {
    'name': 'Alice',
    'email': 'alice@example.com',
    'age': 30,
  }),
]);

// Batch write (atomic)
db.writeTxn([
  WriteOp.put('users', data: {'name': 'Bob', 'email': 'bob@example.com'}),
  WriteOp.put('users', data: {'name': 'Charlie', 'email': 'charlie@example.com'}),
]);
```

### 3. Query Documents

```dart
// Get all
final users = db.findAll('public.users');

// With filter
final adults = db.findAll('public.users', filter: {
  'Condition': {'GreaterThanOrEqual': {'field': 'age', 'value': 18}}
});

// With sort and pagination
final page = db.findAll('public.users',
  sort: [{'field': 'name', 'direction': 'Asc'}],
  offset: 0,
  limit: 10,
);

// Get by ID
final user = db.get('public.users', 1);
```

### 4. Use the Query Builder

```dart
// With code-generated filter extensions
final results = userDao.findWhere(
  (q) => q
    .nameContains('alice')
    .ageGreaterThan(18)
    .sortByName()
    .limit(10),
);
```

See [Query System](query-system.md) for full details.

### 5. Close the Database

```dart
db.close();
```

## With Code Generation

### 1. Define Models

```dart
// lib/models/user.dart
import 'package:nodedb/nodedb.dart';

part 'user.nodedb.g.dart';

@collection
class User {
  String name;
  @Index(unique: true)
  String email;
  int age;

  User({required this.name, required this.email, this.age = 0});
}
```

### 2. Run Code Generation

```bash
dart run build_runner build
```

This generates `user.nodedb.g.dart` with:
- `UserDao` — typed CRUD (create, findById, findWhere, updateById, deleteById)
- `UserFilterExtension` — typed filter methods (nameContains, emailEqualTo, etc.)
- Serialization helpers (toMap, fromMap)
- Schema metadata

### 3. Use Generated DAO

```dart
final users = UserDao(db.nosql, db.provenance);

// Create
users.create(User(name: 'Alice', email: 'alice@example.com', age: 30));

// Query with typed filters
final results = users.findWhere(
  (q) => q.ageGreaterThan(25).sortByName(),
);

// Update
users.updateById(someId, {'age': 31});
```

See [Code Generation](code-generation.md) for all annotations and generated output.

## Optional Engines

Enable additional engines when opening:

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'main',
  graphEnabled: true,           // Graph engine (nodes + edges)
  vectorConfig: VectorOpenConfig(
    dimension: 128,
    metric: DistanceMetric.cosine,
  ),
  dacEnabled: true,             // Access control
  provenanceEnabled: true,      // Data lineage
  keyResolverEnabled: true,     // Public key registry
);
```

### With Mesh Networking

To enable transport (peer-to-peer sync), create a `DatabaseMesh`:

```dart
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(meshName: 'my-mesh'),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',
    mdnsEnabled: true,
  ),
);

final db = NodeDB.open(
  directory: '$baseDir/data',
  databaseName: 'main',
  sharingStatus: 'full',
  mesh: mesh,
  provenanceEnabled: true,
);
```

## Next Steps

- [Architecture Overview](architecture.md) — understand the system design
- [NoSQL Engine](nosql-engine.md) — deep dive into document storage
- [Code Generation](code-generation.md) — annotation reference
- [Transport Layer](transport.md) — networking and pairing
