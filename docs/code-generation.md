# Code Generation

[← Back to Index](README.md)

NodeDB uses `build_runner` to generate typed DAOs, filter extensions, serialization helpers, and preference accessors from annotated Dart classes.

## Setup

### Dependencies

```yaml
# pubspec.yaml
dependencies:
  nodedb: ^1.0.0

dev_dependencies:
  nodedb_generator: ^1.0.0
  build_runner: ^2.4.0
```

### Build Configuration

```yaml
# build.yaml
targets:
  $default:
    builders:
      nodedb_generator|nodedb:
        generate_for:
          - lib/models/**
```

### Running

```bash
dart run build_runner build
# or watch mode:
dart run build_runner watch
```

## Annotations Reference

### @collection

Marks a class as a NoSQL collection:

```dart
@collection
class User {
  String name;
  @Index(unique: true)
  String email;
  int age;
  DateTime? avatarUpdatedAt;

  User({required this.name, required this.email, this.age = 0});
}
```

Options:
- `@collection` — default schema (`public`), String ID with UUID v7 auto-gen
- `@collection(schema: 'analytics')` — custom schema
- `@collection(singleton: true)` — singleton collection (fixed ID=1)

**Generates:**
- `UserDao` — typed CRUD methods
- `UserFilterExtension` on `FilterQuery<User>` — typed filter/sort methods
- `_$UserFromMap()` / `_$UserToMap()` — serialization
- `userSchema` — schema metadata

### @node

Marks a class as a graph node:

```dart
@node
class Person {
  String name;
  int age;
  Person({required this.name, this.age = 0});
}
```

**Generates:** Same as `@collection` but for graph operations.

### @Edge

Marks a class as a graph edge with typed endpoints:

```dart
@Edge(from: Person, to: Person)
class Knows {
  int since;
  String? relationship;
  Knows({required this.since});
}
```

**Generates:** Edge-specific DAO with source/target typing.

### @embedded

Marks a class as a nested object (no separate collection):

```dart
@embedded
class Address {
  String street;
  String city;
  String zip;
}

@collection
class User {
  String name;
  Address address; // Nested, not a separate collection
}
```

### @preferences

Marks a class as a typed preference store:

```dart
@preferences
class AppSettings {
  String theme;
  bool notificationsEnabled;
  int refreshInterval;
  String? lastSyncDate;
}
```

**Generates:**
```dart
class AppSettingsPrefs {
  // Getters (nullable return)
  String? getTheme();
  bool? getNotificationsEnabled();
  int? getRefreshInterval();

  // Setters
  void setTheme(String value, {bool shareable = false});
  void setNotificationsEnabled(bool value, {bool shareable = false});

  // Removers
  bool removeTheme();

  // Utilities
  List<String> allKeys();
  void removeAll();
  List<Map<String, dynamic>> shareableEntries();
}
```

## Field Annotations

### @Index

Create an index on a field:

```dart
@Index()                          // Simple index
@Index(unique: true)              // Unique constraint
@Index(type: IndexType.fullText)  // Full-text search
@Index(composite: ['field2'])     // Composite index
```

### @VectorField

Mark a field as a vector embedding:

```dart
@VectorField(dimensions: 128, metric: DistanceMetric.cosine)
List<double> embedding;
```

### @Enumerated

Store an enum as a string:

```dart
@Enumerated
OrderStatus status; // Stored as 'pending', 'confirmed', etc.
```

### @Trimmable / @neverTrim

Control automatic record trimming:

```dart
@collection
@Trimmable(policy: 'default')
class LogEntry { ... }

@collection
@neverTrim
class AuditRecord { ... }
```

Generates: `trim()`, `recommendTrim()`, `setTrimPolicy()` methods on the DAO.

### @Trigger

Declare database triggers:

```dart
@collection
@Trigger(event: 'insert', timing: 'after', name: 'log_new_user')
@Trigger(event: 'update', timing: 'before', name: 'validate_user')
class User { ... }
```

Generates: `registerDeclaredTriggers()` method on the DAO.

### @ProvenanceConfig

Configure provenance tracking per collection:

```dart
@collection
@ProvenanceConfig(confidenceDecayHalfLifeDays: 30)
class SensorReading { ... }
```

Generates: `findAllWithProvenance()` returning `List<WithProvenance<T>>`.

### @Access

Field-level access control:

```dart
@Access(permission: 'redact')
String socialSecurityNumber;
```

### @Shareable

Federation sharing status:

```dart
@Shareable(status: 'read_write')
class Product { ... }
```

### @nodeLink / @documentLink

Graph relationship references:

```dart
@nodeLink
int authorNodeId;

@documentLink
String relatedDocId;
```

### @noDao

Skip DAO generation for a class:

```dart
@collection
@noDao
class InternalMetadata { ... }
```

## Generated Output

For a `@collection` class `User`, the generator produces a `.nodedb.g.dart` part file with:

### 1. Schema

```dart
const userSchema = NodeDbSchema(
  name: 'User',
  schema: 'public',
  singleton: false,
  type: 'collection',
  fields: [
    SchemaField('id', 'string'),
    SchemaField('name', 'string'),
    SchemaField('email', 'string', unique: true),
    SchemaField('age', 'int'),
  ],
);
```

### 2. DAO Base

```dart
abstract class UserDaoBase {
  NoSqlEngine get _engine;
  ProvenanceEngine? get _provenanceEngine;
  String get collectionName => 'public.users';

  User? findById(String id);
  void create(User item);
  void createWithCache(User item, CacheConfig cache);
  void createAll(List<User> items);
  void save(User item);
  void saveWithCache(User item, CacheConfig cache);
  List<User> findAll({int? offset, int? limit});
  List<User> findWhere(FilterQuery<User> Function(FilterQuery<User>) builder);
  int count();
  void updateById(String id, Map<String, dynamic> updates);
  void deleteById(String id);
  void deleteWhere(FilterQuery<User> Function(FilterQuery<User>) builder);
  int sweepExpired(); // delete expired cached records
}
```

### 3. Filter Extensions

```dart
extension UserFilterExtension on FilterQuery<User> {
  FilterQuery<User> nameEqualTo(String value) => equalTo('name', value);
  FilterQuery<User> nameContains(String value) => contains('name', value);
  FilterQuery<User> nameStartsWith(String value) => startsWith('name', value);
  FilterQuery<User> nameEndsWith(String value) => endsWith('name', value);
  FilterQuery<User> nameInList(List<String> values) => inList('name', values);
  FilterQuery<User> nameNotInList(List<String> values) => notInList('name', values);

  FilterQuery<User> ageEqualTo(int value) => equalTo('age', value);
  FilterQuery<User> ageGreaterThan(int value) => greaterThan('age', value);
  FilterQuery<User> ageLessThan(int value) => lessThan('age', value);
  FilterQuery<User> ageBetween(int low, int high) => between('age', low, high);

  FilterQuery<User> sortByName({bool desc = false}) => sortBy('name', desc: desc);
  FilterQuery<User> sortByAge({bool desc = false}) => sortBy('age', desc: desc);
}
```

### 4. Serialization

```dart
User _$UserFromMap(Map<String, dynamic> map) => User(
  name: map['name'] as String,
  email: map['email'] as String,
  age: map['age'] as int? ?? 0,
);

Map<String, dynamic> _$UserToMap(User instance) => {
  'name': instance.name,
  'email': instance.email,
  'age': instance.age,
};
```

## Example: Full Model File

```dart
// lib/models/user.dart
import 'package:nodedb/nodedb.dart';

part 'user.nodedb.g.dart';

@collection
@Trimmable(policy: 'default')
@Trigger(event: 'insert', timing: 'after', name: 'welcome_email')
class User {
  String name;

  @Index(unique: true)
  String email;

  int age;
  DateTime? lastLoginAt;

  User({required this.name, required this.email, this.age = 0});
}

// Usage in database setup:
class UserDao extends UserDaoBase {
  final NoSqlEngine _engine;
  final ProvenanceEngine? _provenanceEngine;

  UserDao(this._engine, this._provenanceEngine);
}
```

## Related Pages

- [Query System](query-system.md) — using generated filter extensions
- [NoSQL Engine](nosql-engine.md) — singletons, preferences, triggers
- [Data Provenance](provenance.md) — `@ProvenanceConfig` annotation
- [Getting Started](getting-started.md) — setup and first model
