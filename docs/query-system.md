# Query System

[← Back to Index](README.md)

NodeDB provides a structured query system with filters, sorting, pagination, and federated query modes.

## Filter DSL

Filters are expressed as nested maps matching Rust enum variants. The format is:

```
{"Condition": {"<Operator>": {"field": "<name>", "value": <value>}}}
```

### Condition Operators

| Operator | Example | Description |
|----------|---------|-------------|
| `EqualTo` | `{'field': 'name', 'value': 'Alice'}` | Exact match |
| `NotEqualTo` | `{'field': 'status', 'value': 'banned'}` | Not equal |
| `GreaterThan` | `{'field': 'age', 'value': 25}` | Strict greater than |
| `GreaterThanOrEqual` | `{'field': 'age', 'value': 18}` | Greater or equal |
| `LessThan` | `{'field': 'price', 'value': 100.0}` | Strict less than |
| `LessThanOrEqual` | `{'field': 'score', 'value': 50}` | Less or equal |
| `Contains` | `{'field': 'name', 'value': 'ali'}` | Substring match |
| `StartsWith` | `{'field': 'email', 'value': 'admin'}` | Prefix match |
| `EndsWith` | `{'field': 'email', 'value': '@example.com'}` | Suffix match |
| `IsNull` | `{'field': 'deletedAt'}` | Field is null/missing |
| `IsNotNull` | `{'field': 'avatar'}` | Field exists |
| `Between` | `{'field': 'age', 'low': 18, 'high': 65}` | Range (inclusive) |

### Compound Filters

```dart
// AND — all conditions must match
{'And': [
  {'Condition': {'GreaterThan': {'field': 'age', 'value': 18}}},
  {'Condition': {'Contains': {'field': 'name', 'value': 'ali'}}},
]}

// OR — any condition must match
{'Or': [
  {'Condition': {'EqualTo': {'field': 'role', 'value': 'admin'}}},
  {'Condition': {'EqualTo': {'field': 'role', 'value': 'moderator'}}},
]}

// Nested
{'And': [
  {'Or': [
    {'Condition': {'EqualTo': {'field': 'status', 'value': 'active'}}},
    {'Condition': {'EqualTo': {'field': 'status', 'value': 'pending'}}},
  ]},
  {'Condition': {'GreaterThan': {'field': 'score', 'value': 50}}},
]}
```

### Sorting

```dart
sort: [
  {'field': 'name', 'direction': 'Asc'},
  {'field': 'createdAt', 'direction': 'Desc'},
]
```

### Pagination

```dart
offset: 20,  // Skip first 20 results
limit: 10,   // Return at most 10 results
```

## FilterQuery Builder (Dart)

The `FilterQuery<T>` class provides a fluent, type-safe API:

```dart
final query = FilterQuery<User>()
  .equalTo('name', 'Alice')
  .greaterThan('age', 18)
  .contains('email', '@example.com')
  .between('score', 50, 100)
  .isNotNull('avatar')
  .sortBy('name')
  .sortBy('age', desc: true)
  .offset(0)
  .limit(20);

final filter = query.build(); // Returns the filter map for FFI
```

### Available Methods

**Conditions:**
- `equalTo(field, value)`, `notEqualTo(field, value)`
- `greaterThan(field, value)`, `greaterThanOrEqual(field, value)`
- `lessThan(field, value)`, `lessThanOrEqual(field, value)`
- `contains(field, value)`, `startsWith(field, value)`, `endsWith(field, value)`
- `isNull(field)`, `isNotNull(field)`
- `between(field, low, high)`
- `inList(field, values)` — expands to `Or([EqualTo, EqualTo, ...])`
- `notInList(field, values)` — expands to `And([NotEqualTo, NotEqualTo, ...])`

**Combinators:**
- `and(builder)` — merge conditions with AND
- `or(builder)` — create OR group

**Sorting & Pagination:**
- `sortBy(field, {bool desc = false})`
- `offset(n)`, `limit(n)`

**Query Flags:**
- `distinct()` — deduplicate results (Dart-side)
- `withProvenance()` — attach provenance envelopes
- `withFederation()` / `acrossPeers()` — query federated peers
- `withAiQuery()` — fall back to AI adapter when local empty

**Debugging:**
- `describe()` — human-readable query description

## Code-Generated Filter Extensions

When using `@collection` with code generation, typed filter methods are generated:

```dart
// Generated for User { String name; int age; DateTime createdAt; }
extension UserFilterExtension on FilterQuery<User> {
  // String fields
  FilterQuery<User> nameEqualTo(String value) => equalTo('name', value);
  FilterQuery<User> nameContains(String value) => contains('name', value);
  FilterQuery<User> nameStartsWith(String value) => startsWith('name', value);
  FilterQuery<User> nameEndsWith(String value) => endsWith('name', value);
  FilterQuery<User> nameInList(List<String> values) => inList('name', values);

  // Numeric fields
  FilterQuery<User> ageEqualTo(int value) => equalTo('age', value);
  FilterQuery<User> ageGreaterThan(int value) => greaterThan('age', value);
  FilterQuery<User> ageBetween(int low, int high) => between('age', low, high);

  // DateTime fields
  FilterQuery<User> createdAtGreaterThan(DateTime value) => ...;
  FilterQuery<User> createdAtBetween(DateTime low, DateTime high) => ...;

  // Nullable fields
  FilterQuery<User> avatarIsNull() => isNull('avatar');
  FilterQuery<User> avatarIsNotNull() => isNotNull('avatar');

  // Sorting
  FilterQuery<User> sortByName({bool desc = false}) => sortBy('name', desc: desc);
  FilterQuery<User> sortByAge({bool desc = false}) => sortBy('age', desc: desc);
}
```

## Federated Queries

Query across connected peers:

```dart
// Via the facade
final results = db.findAllFederated(
  'public.products',
  filter: {'Condition': {'Contains': {'field': 'name', 'value': 'Laptop'}}},
);

// Returns List<FederatedResult<Document>>
for (final r in results) {
  print('${r.data.data['name']} from peer: ${r.sourcePeerId}');
}
```

Or via the query builder:

```dart
final results = productDao.findWhere(
  (q) => q.nameContains('Laptop').withFederation(),
);
```

See [Federation & Mesh](federation.md) for networking details.

## Full Query Pipeline

The `findAllFull()` method chains: local → federation → AI fallback:

```dart
final results = db.findAllFull(
  'public.products',
  filter: filter,
);
```

1. Query local database
2. If federation enabled: query connected peers, merge results
3. If AI adapter configured and results empty: invoke AI query
4. Return combined results with provenance metadata

## Related Pages

- [NoSQL Engine](nosql-engine.md) — underlying storage
- [Code Generation](code-generation.md) — generating filter extensions
- [Federation & Mesh](federation.md) — cross-device queries
- [AI Integration](ai-integration.md) — AI query fallback
