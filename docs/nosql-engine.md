# NoSQL Engine

[← Back to Index](README.md)

The NoSQL engine is NodeDB's primary data store — a schema-optional document database backed by [sled](https://github.com/spacejam/sled) with optional AES-256-GCM encryption.

## Core Concepts

### Documents

A `Document` is the fundamental unit of storage:

```dart
class Document {
  final int id;                    // Internal sled key (i64)
  final String collection;        // e.g., 'public.users'
  final Map<String, dynamic> data; // Arbitrary key-value data
  final DateTime createdAt;
  final DateTime updatedAt;
}
```

- **String IDs**: By default, collections use String IDs (UUID v7, auto-generated). The UUID is stored in `data['id']`.
- **Integer IDs**: Legacy support. Sled always uses i64 keys internally.

### Collections

Collections are logical groupings of documents within a schema:

- **Qualified names**: `schema.collection` (e.g., `public.users`)
- **Default schema**: `public` (can be overridden via `@collection(schema: 'custom')`)
- **Reserved schema**: `security` — write-protected for system use

### Schemas

Schema metadata is stored in a `__meta__` tree:

- Key format: `schema::collection` (e.g., `public::users`)
- `SchemaEntry` stores: tree_name, created_at, singleton flag, collection_type
- Legacy entries (without `::`) auto-migrate to `public::name`
- Schema fingerprint: SHA-256 of sorted meta keys (used for mesh consistency)

## CRUD Operations

### Write Transaction

All writes go through atomic transactions:

```dart
db.writeTxn([
  WriteOp.put('users', data: {'name': 'Alice', 'email': 'alice@example.com'}),
  WriteOp.put('users', data: {'name': 'Bob', 'email': 'bob@example.com'}),
  WriteOp.delete('users', id: 42),
]);
```

`WriteOp` supports:
- `put(collection, data, id?)` — insert or update
- `delete(collection, id)` — remove by ID
- `singletonPut(collection, data)` — update singleton
- `prefSet(store, key, value, shareable?, conflictResolution?)` — set preference

### Queries

```dart
// Get by ID
final doc = db.get('public.users', 1);

// Find all with optional filter/sort/pagination
final docs = db.findAll('public.users',
  filter: {'Condition': {'Contains': {'field': 'name', 'value': 'alice'}}},
  sort: [{'field': 'name', 'direction': 'Asc'}],
  offset: 0,
  limit: 20,
);

// Count
final count = db.count('public.users');
```

See [Query System](query-system.md) for the full filter DSL.

## Singletons

A singleton collection has exactly one document (ID=1):

```dart
// Create with defaults (only on first call)
db.singletonCreate('app_config', defaults: {
  'theme': 'system',
  'version': 1,
});

// Read
final config = db.singletonGet('app_config');

// Update
db.singletonPut('app_config', data: {'theme': 'dark', 'version': 2});

// Reset to defaults
db.singletonReset('app_config');
```

- Delete and clear are **guarded** (ERR_SINGLETON_DELETE, ERR_SINGLETON_CLEAR)
- Defaults are persisted in `__singleton_defaults__` tree for reset across reopens
- Declare via `@collection(singleton: true)` for code generation

## Preferences

Encrypted key-value storage with per-key HKDF-derived encryption:

```dart
// Set a preference
db.prefSet('user_prefs', 'theme', 'dark');

// Get
final theme = db.prefGet('user_prefs', 'theme'); // 'dark' or null

// List keys
final keys = db.prefKeys('user_prefs'); // ['theme', ...]

// Remove
db.prefRemove('user_prefs', 'theme');

// Shareable entries (for federation sync)
final entries = db.prefShareable('user_prefs');
```

- Encryption: `hkdf_derive_key(dek, "prefs:" + key)` → AES-256-GCM per key
- Plain msgpack when no DEK (unencrypted database)
- Conflict resolution: `LastWriteWins`, `LocalWins`, `RemoteWins`, `HighestConfidence`, `Manual`
- Federation: `PreferenceSyncPayload` in wire protocol

## Triggers

Database triggers fire on insert/update/delete/clear:

```dart
// Register
final triggerId = db.registerTrigger(
  'public.users',
  event: 'insert',
  timing: 'after',
  name: 'log_new_user',
);

// Disable/enable
db.setTriggerEnabled(triggerId, false);

// Unregister
db.unregisterTrigger(triggerId);
```

**Timing**:
- `before` — can modify data or abort (last `instead` wins)
- `after` — notification only (abort ignored)
- `instead` — replaces the write entirely

**Pipeline**: `instead` (last wins, skip before/after) → `before` (chain modifications) → write → `after`

**Reentrancy**: Thread-local depth counter, max 8 levels.

**Mesh**: Trigger notifications are broadcast to connected peers via `TriggerNotificationPayload`.

## Access History & Trimming

### Access History

Track read access to documents:

```dart
final history = db.accessHistoryQuery(collection: 'public.users');
final count = db.accessHistoryCount();
db.accessHistoryTrim(retentionSecs: 86400 * 30); // Keep 30 days
```

### Trimming

Automatic record pruning based on configurable policies:

```dart
// Get recommendations (dry run)
final rec = db.recommendTrim('public.logs', policy: trimPolicy);

// Execute trim
final report = db.trim('public.logs', policy: trimPolicy);

// Trim all trimmable collections
final reports = db.trimAll();

// Protect specific records
db.trimConfigSetRecordNeverTrim('public.users', recordId: 42);
```

Policies: `LastModified`, `LastAccessed`, `RecordCount`, `FileSize`.

Annotations: `@Trimmable(policy: 'default')`, `@neverTrim`.

### Record Cache / TTL

Per-record cache configuration with automatic expiry. Records can be given a TTL (time-to-live) that causes them to be automatically removed after a specified duration.

**Cache modes:**
- `CacheMode.expireAfterWrite` — TTL measured from last `updatedAt` timestamp
- `CacheMode.expireAfterCreate` — TTL measured from `createdAt` timestamp

```dart
// Create a record with a 60-second TTL (expires 60s after last write)
db.nosql.writeTxn([
  WriteOp.put('public.search_cache',
    data: {'query': 'flutter db', 'results': '...'},
    cache: CacheConfig(
      mode: CacheMode.expireAfterWrite,
      ttl: Duration(seconds: 60),
    ),
  ),
]);

// Set cache config on an existing record
db.nosql.setRecordCache('public.search_cache', recordId, CacheConfig(
  mode: CacheMode.expireAfterWrite,
  ttl: Duration(minutes: 5),
));

// Check cache config
final config = db.nosql.getRecordCache('public.search_cache', recordId);

// Clear cache config (record becomes permanent)
db.nosql.clearRecordCache('public.search_cache', recordId);

// Sweep expired records in a collection
final deletedCount = db.nosql.sweepExpired('public.search_cache');

// Sweep all expired records across all collections
final totalDeleted = db.nosql.sweepAllExpired();
```

**Lazy eviction**: When reading a record via `get()`, if it has an expired cache config, the record is automatically deleted and `null` is returned. This provides cleanup without requiring explicit sweep calls.

**Generated DAO methods** (for `@collection` models):
```dart
// Create with cache TTL
userDao.createWithCache(user, CacheConfig(ttl: Duration(hours: 1)));
userDao.saveWithCache(user, CacheConfig(ttl: Duration(hours: 1)));
userDao.sweepExpired(); // returns count deleted
```

**Storage**: Cache metadata is stored in a separate `__record_cache_config__` sled tree (not inside the Document struct), keyed by `"{meta_key}\0{record_id}"`.

## Schema Management

```dart
// Create schema
db.nosql.createSchema('analytics');

// List schemas
final schemas = db.nosql.listSchemas();

// Schema fingerprint (SHA-256 of sorted meta keys)
final fingerprint = db.nosql.schemaFingerprint();

// Run migration
db.nosql.runMigration(NodeMigration(
  version: 2,
  operations: [
    MigrationOp.renameTree('old_name', 'new_name'),
    MigrationOp.dropTree('deprecated'),
  ],
));
```

## Rust Implementation

**Crate**: `nodedb-nosql` → depends on `nodedb-storage`, `nodedb-crypto`

Key types:
- `Database` — main facade (path-based or engine-based opening)
- `Collection` — per-collection document manager
- `Query`, `Filter`, `FilterCondition` — query DSL
- `TriggerRegistry` — trigger management
- `PreferencesStore` — preference storage
- `AccessHistoryStore` — access tracking

## Related Pages

- [Query System](query-system.md) — filter DSL and query builder
- [Code Generation](code-generation.md) — `@collection`, `@singleton`, `@preferences` annotations
- [Security](security.md) — encryption and access control
- [Provenance](provenance.md) — per-record origin tracking
