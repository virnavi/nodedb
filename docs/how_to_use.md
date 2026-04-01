# How to Use NodeDB

[← Back to Index](README.md)

A practical guide to using NodeDB in your Dart/Flutter application — from setup to mesh networking.

## Table of Contents

- [Installation](#installation)
- [Opening a Database](#opening-a-database)
- [Defining Models](#defining-models)
- [Code Generation](#code-generation)
- [CRUD Operations](#crud-operations)
- [Querying Data](#querying-data)
- [Mesh Networking](#mesh-networking)
- [Provenance Tracking](#provenance-tracking)
- [Singletons and Preferences](#singletons-and-preferences)
- [Graph Engine](#graph-engine)
- [Debug Inspector](#debug-inspector)
- [Closing and Cleanup](#closing-and-cleanup)

---

## Installation

### Dependencies

```yaml
# pubspec.yaml
dependencies:
  nodedb:
    path: packages/nodedb
  nodedb_flutter_libs:
    path: packages/nodedb_flutter_libs  # Pre-compiled native binaries

dev_dependencies:
  nodedb_generator:
    path: packages/nodedb_generator
  build_runner: ^2.4.0
```

### Build the Native Library

```bash
cd rust/
cargo build --release -p nodedb-ffi
```

This produces `target/release/libnodedb_ffi.dylib` (macOS), `.so` (Linux), or `.dll` (Windows).

---

## Opening a Database

### Local-Only (Minimal)

```dart
import 'package:nodedb/nodedb.dart';

final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'my-app',
);
```

### With Optional Engines

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'my-app',
  graphEnabled: true,             // Property graph
  provenanceEnabled: true,        // Data lineage tracking
  dacEnabled: true,               // Access control
  keyResolverEnabled: true,       // Public key registry
  vectorConfig: VectorOpenConfig( // Vector search
    dimension: 128,
    metric: DistanceMetric.cosine,
  ),
);
```

### With Mesh Networking

Mesh networking enables peer-to-peer sync across devices. Transport configuration lives at the mesh level, not on individual databases.

```dart
// 1. Create a shared mesh coordinator
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(
    meshName: 'my-app',
    meshSecret: 'optional-hmac-secret',   // Authenticates gossip
    ownerPrivateKeyHex: '...',            // Shared encryption key
  ),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',           // Base port
    mdnsEnabled: true,                     // Auto-discover on LAN
  ),
);

// 2. Open databases through the mesh
final usersDb = NodeDB.open(
  directory: '$baseDir/users',
  databaseName: 'users',
  mesh: mesh,                              // Port 9400 (auto-allocated)
  provenanceEnabled: true,
);

final productsDb = NodeDB.open(
  directory: '$baseDir/products',
  databaseName: 'products',
  mesh: mesh,                              // Port 9401 (auto-incremented)
  provenanceEnabled: true,
);
```

Each database gets its own transport (required by the Rust FFI), but they share the mesh's federation engine and transport configuration. Ports are auto-allocated from the base address.

---

## Defining Models

### Collections

```dart
import 'package:nodedb/nodedb.dart';

part 'user.nodedb.g.dart';

@collection
class User {
  String name;

  @Index(unique: true)
  String email;

  int age;
  DateTime? lastLogin;

  User({required this.name, required this.email, this.age = 0});
}
```

### Graph Nodes and Edges

```dart
@node
class Person {
  String name;
  Person({required this.name});
}

@Edge(from: Person, to: Person)
class Follows {
  DateTime since;
  Follows({required this.since});
}
```

### Singletons

```dart
@Collection(singleton: true)
class AppConfig {
  String theme;
  bool darkMode;
  AppConfig({this.theme = 'system', this.darkMode = false});
}
```

### Preferences

```dart
@preferences
class UserPrefs {
  String theme;
  bool notificationsEnabled;
  int fontSize;
  UserPrefs({
    this.theme = 'system',
    this.notificationsEnabled = true,
    this.fontSize = 14,
  });
}
```

### Views (Read-Only Cross-Database)

```dart
@NodeDBView(sources: [
  ViewSource(collection: 'public.users', database: 'users'),
  ViewSource(collection: 'public.products', database: 'products'),
])
class UserProductView {
  String userName;
  String productName;
  UserProductView({required this.userName, required this.productName});
}
```

### Additional Annotations

```dart
@collection
class Article {
  String title;
  String body;

  @Index()
  String authorId;

  @Jsonb()
  Map<String, dynamic> metadata;     // JSONB path queries

  @Shareable()
  String category;                    // Shared via mesh

  @Trimmable()
  DateTime publishedAt;               // Can be auto-trimmed

  List<String> tags;                  // Array operators

  Article({required this.title, required this.body, required this.authorId,
           this.metadata = const {}, this.category = '', required this.publishedAt,
           this.tags = const []});
}
```

---

## Code Generation

### Setup

```yaml
# build.yaml
targets:
  $default:
    builders:
      nodedb_generator|nodedb:
        generate_for:
          - lib/models/**
```

### Run

```bash
dart run build_runner build
# or watch mode:
dart run build_runner watch
```

This generates for each model:
- **Schema** — collection metadata with field types
- **Serialization** — `toMap()` / `fromMap()` / `fromDocument()`
- **Filter extensions** — typed query methods (e.g., `nameContains()`, `ageGreaterThan()`)
- **DAO base class** — typed CRUD (create, findById, findWhere, updateById, deleteById)
- **Concrete DAO** — extensible class for custom queries
- **DAO registry** — `db.dao.users`, `db.dao.products` extension on `NodeDB`

---

## CRUD Operations

### Using Generated DAOs

```dart
final users = UserDao(db.nosql, db.provenance, db.notifier);

// Create
final user = users.create(User(
  name: 'Alice',
  email: 'alice@example.com',
  age: 30,
));
print(user.id); // UUID v7 string

// Create multiple
users.createAll([
  User(name: 'Bob', email: 'bob@example.com'),
  User(name: 'Charlie', email: 'charlie@example.com'),
]);

// Read by ID
final alice = users.findById(user.id);

// Update
users.updateById(user.id, {'age': 31});

// Delete
users.deleteById(user.id);

// Count
final count = users.count();
```

### Using Raw NoSQL API

```dart
// Write transaction (atomic)
db.writeTxn([
  WriteOp.put('users', data: {'name': 'Alice', 'email': 'alice@example.com'}),
  WriteOp.put('users', data: {'name': 'Bob', 'email': 'bob@example.com'}),
]);

// Get by ID
final doc = db.get('public.users', 1);

// Find all
final all = db.findAll('public.users');

// Find with filter + sort + pagination
final page = db.findAll('public.users',
  filter: {'Condition': {'GreaterThan': {'field': 'age', 'value': 25}}},
  sort: [{'field': 'name', 'direction': 'Asc'}],
  offset: 0,
  limit: 10,
);
```

---

## Querying Data

### Typed Query Builder

```dart
// Simple filter
final adults = users.findWhere(
  (q) => q.ageGreaterThanOrEqualTo(18),
);

// Compound filter with sort and limit
final results = users.findWhere(
  (q) => q
    .nameContains('ali')
    .ageGreaterThan(20)
    .sortByName()
    .limit(10),
);

// Find first match
final first = users.findFirst(
  (q) => q.emailEqualTo('alice@example.com'),
);
```

### JSONB Queries

For `@Jsonb()` fields:

```dart
// Path-based equality
products.findWhere(
  (q) => q.metadataPathEquals('color', 'red'),
);

// Key existence
products.findWhere(
  (q) => q.metadataHasKey('weight'),
);

// Map containment
products.findWhere(
  (q) => q.metadataContains({'color': 'red'}),
);
```

### Array Queries

For `List` fields:

```dart
// Array contains element
articles.findWhere(
  (q) => q.tagsContains('dart'),
);

// Array overlaps with list
articles.findWhere(
  (q) => q.tagsOverlaps(['dart', 'flutter']),
);
```

### Reactive Streams (Watch)

```dart
// Watch all users (re-queries on any change)
users.watchAll().listen((userList) {
  print('Users updated: ${userList.length}');
});

// Watch a specific user
users.watchById(userId).listen((user) {
  if (user != null) print('User changed: ${user.name}');
});

// Watch with filter
users.watchWhere((q) => q.ageGreaterThan(18)).listen((adults) {
  print('Adults: ${adults.length}');
});
```

---

## Mesh Networking

### Setting Up a Mesh

```dart
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(meshName: 'my-app'),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',
    mdnsEnabled: true,
  ),
);
```

### Peer Management

```dart
// Add a peer (auto-added to "all" group)
final peer = mesh.addPeer('Bob\'s Phone', 'wss://192.168.1.5:9400');

// List all peers
final peers = mesh.allPeers();

// Remove a peer (removed from all groups, then deleted)
mesh.removePeer(peer.id);

// Groups
final family = mesh.addGroup('family');
mesh.addMember(family.id, peer.id);
final groups = mesh.groupsForPeer(peer.id);
```

### Federated Queries

```dart
// Search across all connected peers
final results = db.findAllFederated(
  'public.products',
  filter: {'Condition': {'Contains': {'field': 'name', 'value': 'Laptop'}}},
);

for (final r in results) {
  print('${r.data.data['name']} from ${r.sourcePeerId}');
  // sourcePeerId is 'local' or a peer UUID
}

// Targeted mesh query (specific database)
final transport = db.transport!;
final remote = transport.meshQuery(
  database: 'products',
  queryType: 'nosql',
  queryData: {'collection': 'public.products'},
  timeoutSecs: 10,
);
```

### Manual Connection (QR Pairing)

```dart
// Connect to a peer directly
db.transport?.connect('wss://192.168.1.5:9400');

// With pairing enabled:
final pending = transport.pendingPairings();
transport.approvePairing(peerId);
transport.rejectPairing(peerId);

// List paired devices
final devices = transport.pairedDevices();
transport.removePairedDevice(peerId);
```

---

## Provenance Tracking

Provenance tracks the origin and confidence of every record.

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'main',
  provenanceEnabled: true,
);

// Attach provenance to a record
db.provenance!.attach(
  'public.users',
  recordId,
  'user-1',                    // Source ID
  confidence: 0.95,
  sourceType: 'user',
);

// Query with provenance
final results = db.findAllWithProvenance('public.users');
for (final r in results) {
  print('${r.data.data['name']} — confidence: ${r.provenance?.confidence}');
}

// Get provenance for a specific record
final wp = db.getWithProvenance('public.users', id);
print(wp?.provenance?.verificationStatus);
```

### Generated DAO Provenance Support

```dart
// DAOs with provenance automatically track writes
final users = UserDao(db.nosql, db.provenance, db.notifier);
users.create(User(name: 'Alice', email: 'alice@example.com'));
// Provenance envelope auto-created

final withProv = users.findAllWithProvenance();
```

---

## Singletons and Preferences

### Singletons

A singleton is a collection with exactly one record:

```dart
// Create singleton with defaults
db.singletonCreate('app_config', {'theme': 'system', 'darkMode': false});

// Read
final config = db.singletonGet('app_config');
print(config.data['theme']);

// Update
db.singletonPut('app_config', {'theme': 'dark', 'darkMode': true});

// Reset to defaults
db.singletonReset('app_config');
```

### Preferences (Encrypted Key-Value)

```dart
// Set a preference (encrypted at rest if owner key is set)
db.prefSet('user_prefs', 'theme', 'dark');
db.prefSet('user_prefs', 'fontSize', 16, shareable: true);

// Get
final theme = db.prefGet('user_prefs', 'theme');

// Remove
db.prefRemove('user_prefs', 'theme');

// List keys
final keys = db.prefKeys('user_prefs');
```

### Generated Typed Preferences

```dart
// From @preferences annotation
final prefs = UserPrefsPrefs(db.nosql);

prefs.setTheme('dark');
prefs.setNotificationsEnabled(false);
prefs.setFontSize(16);

final theme = prefs.getTheme();     // String?
final enabled = prefs.getNotificationsEnabled(); // bool?
```

---

## Graph Engine

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'social',
  graphEnabled: true,
);

final graph = db.graph!;

// Create nodes
final alice = graph.addNode({'name': 'Alice', 'label': 'person'});
final bob = graph.addNode({'name': 'Bob', 'label': 'person'});

// Create edge
graph.addEdge(alice.id, bob.id, weight: 1.0, data: {'type': 'follows'});

// Traverse
final bfsResult = graph.bfs(alice.id);     // {nodes: [...], edges: [...]}
final dfsResult = graph.dfs(alice.id);

// Algorithms
final pagerank = graph.pagerank();          // {nodeId: score}
final hasCycle = graph.hasCycle();
final components = graph.connectedComponents();
```

---

## Debug Inspector

### Data Layer

```dart
import 'package:nodedb_inspector/nodedb_inspector.dart';

final inspector = NodeDbInspector(db);

// Full snapshot
final snap = inspector.snapshot();
print(snap['nosql']['totalDocuments']);

// Panel access
inspector.nosql.collectionStats();
inspector.schema.overview();
inspector.graph?.nodePreview(limit: 10);
```

### HTTP/WebSocket Server

```dart
await inspector.start(); // Starts on port 8484
// Open http://localhost:8484 in browser
await inspector.stop();
```

### Flutter Overlay

```dart
import 'package:nodedb_inspector_flutter/nodedb_inspector_flutter.dart';

// In your widget tree:
NodeInspectorOverlay(
  inspector: inspector,
  child: MyApp(),
)
```

---

## Closing and Cleanup

```dart
// Close databases first
usersDb.close();
productsDb.close();

// Then close the mesh (releases shared federation)
mesh.close();
```

Without mesh:

```dart
db.close(); // Closes all engines including its own federation
```

---

## Complete Example

```dart
import 'dart:io';
import 'package:nodedb/nodedb.dart';

void main() {
  final baseDir = Directory.systemTemp.createTempSync('nodedb_demo_').path;

  // Open a local database
  final db = NodeDB.open(
    directory: '$baseDir/data',
    databaseName: 'demo',
    provenanceEnabled: true,
  );

  // Write documents
  db.writeTxn([
    WriteOp.put('users', data: {
      'name': 'Alice',
      'email': 'alice@example.com',
      'age': 30,
    }),
    WriteOp.put('users', data: {
      'name': 'Bob',
      'email': 'bob@example.com',
      'age': 25,
    }),
  ]);

  // Query
  final users = db.findAll('public.users',
    filter: {'Condition': {'GreaterThan': {'field': 'age', 'value': 20}}},
    sort: [{'field': 'name', 'direction': 'Asc'}],
  );

  for (final doc in users) {
    print('${doc.data['name']} (age ${doc.data['age']})');
  }

  print('Total: ${db.count('public.users')} users');

  // Clean up
  db.close();
  Directory(baseDir).deleteSync(recursive: true);
}
```

## Related Pages

- [Getting Started](getting-started.md) — Prerequisites and setup
- [Architecture Overview](architecture.md) — System design
- [Code Generation](code-generation.md) — Full annotation reference
- [Query System](query-system.md) — FilterQuery builder details
- [Transport Layer](transport.md) — Networking configuration
- [Federation & Mesh](federation.md) — Peer management and routing
- [Data Provenance](provenance.md) — Origin tracking
- [Debug Inspector](inspector.md) — Runtime inspection
