/// Annotations for NodeDB code generation.
///
/// Use these annotations on Dart classes to generate schemas,
/// typed query builders, and DAO classes via `nodedb_generator`.
library;

// ── Class-level annotations ─────────────────────────────────────

/// Marks a class as a NodeDB NoSQL collection.
///
/// ```dart
/// @collection
/// class User {
///   int id = 0;
///   late String name;
///   late int age;
/// }
/// ```
const collection = Collection();

class Collection {
  /// Schema name (default: "public").
  final String? schema;

  /// Whether this is a singleton entity (exactly one record).
  final bool singleton;

  /// Whether this collection is included in database backups.
  final bool backup;

  const Collection({this.schema, this.singleton = false, this.backup = true});
}

/// Marks a class as a graph node.
///
/// ```dart
/// @node
/// class Person {
///   int id = 0;
///   late String name;
/// }
/// ```
const node = Node();

class Node {
  final String? schema;
  const Node({this.schema});
}

/// Marks a class as a graph edge with source and target node types.
///
/// ```dart
/// @Edge(from: Person, to: Person)
/// class Knows {
///   int id = 0;
///   late double weight;
/// }
/// ```
class Edge {
  final Type from;
  final Type to;
  final String? schema;
  const Edge({required this.from, required this.to, this.schema});
}

/// Marks a class as an embeddable object (nested inside a collection).
///
/// ```dart
/// @embedded
/// class Address {
///   late String street;
///   late String city;
/// }
/// ```
const embedded = Embedded();

class Embedded {
  const Embedded();
}

/// Marks a field as a JSONB (unstructured Map) field with path query support.
///
/// ```dart
/// @Jsonb()
/// late Map<String, dynamic> metadata;
///
/// @Jsonb(schema: '{"type": "object"}', identifier: 'sku')
/// late Map<String, dynamic> attributes;
/// ```
const jsonb = Jsonb();

class Jsonb {
  /// Optional JSON schema string for validation/documentation.
  final String? schema;

  /// Optional unique identifier field name within the JSONB map.
  final String? identifier;

  const Jsonb({this.schema, this.identifier});
}

/// Marks a class as a JSON model — stored as JSONB with typed conversion.
///
/// The generator produces `_$XFromMap` and `_$XToMap` methods.
/// When used as a field type in a `@collection`, it is serialized as JSONB
/// and supports path-based queries.
///
/// The [name] defaults to `packages/{package}/{ClassName}` when not specified,
/// computed by the generator at build time.
///
/// ```dart
/// @JsonModel()
/// class ProductAttributes {
///   late String color;
///   late int weight;
///   String? material;
/// }
///
/// @JsonModel(name: 'custom/path/ProductAttrs')
/// class ProductAttributes { ... }
/// ```
class JsonModel {
  /// Fully qualified name for this JSON model.
  /// Defaults to `packages/{package}/{ClassName}` at generation time.
  final String? name;

  const JsonModel({this.name});
}

/// Marks a field as an enumerated type stored as a string.
const enumerated = Enumerated();

class Enumerated {
  const Enumerated();
}

// ── Field-level annotations ─────────────────────────────────────

/// Marks a field as indexed for fast lookups.
///
/// ```dart
/// @Index(unique: true)
/// late String email;
///
/// @Index(type: IndexType.fullText)
/// late String bio;
///
/// @Index(composite: ['lastName'])
/// late String firstName;
/// ```
class Index {
  final IndexType type;
  final bool unique;
  final List<String> composite;
  const Index({
    this.type = IndexType.value,
    this.unique = false,
    this.composite = const [],
  });
}

/// Marks a field as a vector embedding for similarity search.
///
/// ```dart
/// @VectorField(dimensions: 128)
/// late List<double> embedding;
/// ```
class VectorField {
  final int dimensions;
  final String metric;
  const VectorField({required this.dimensions, this.metric = 'cosine'});
}

// ── Provenance / Access / Trim ──────────────────────────────────

/// Configures provenance tracking for a collection or node.
///
/// ```dart
/// @ProvenanceConfig(confidenceDecayHalfLifeDays: 30)
/// @collection
/// class Article { ... }
/// ```
class ProvenanceConfig {
  final int? confidenceDecayHalfLifeDays;
  const ProvenanceConfig({this.confidenceDecayHalfLifeDays});
}

/// Marks a field or class with access control rules.
///
/// ```dart
/// @Access(permission: 'read')
/// late String sensitiveData;
/// ```
class Access {
  final String permission;
  const Access({required this.permission});
}

/// Marks a collection as trimmable with a specified policy.
class Trimmable {
  final String policy;
  const Trimmable({this.policy = 'default'});
}

/// Marks a collection as never trimmable.
const neverTrim = NeverTrim();

class NeverTrim {
  const NeverTrim();
}

/// Declares a trigger on a collection.
///
/// ```dart
/// @Trigger(event: 'insert', timing: 'after', name: 'log_insert')
/// @collection
/// class AuditLog { ... }
/// ```
class Trigger {
  /// Trigger event: 'insert', 'update', 'delete', 'clear'.
  final String event;

  /// Trigger timing: 'before', 'after', 'instead'.
  final String timing;

  /// Optional trigger name for identification.
  final String? name;

  const Trigger({
    required this.event,
    this.timing = 'after',
    this.name,
  });
}

/// Opts out of DAO generation for a class.
const noDao = NoDao();

class NoDao {
  const NoDao();
}

// ── Graph link annotations ──────────────────────────────────────

/// Marks a field as a link to a graph node.
const nodeLink = NodeLink();

class NodeLink {
  const NodeLink();
}

/// Marks a field as a link between a document and a node.
const documentLink = DocumentLink();

class DocumentLink {
  const DocumentLink();
}

/// Link to a single related entity.
class IsLink<T> {
  T? value;
  IsLink();
}

/// Link to multiple related entities.
class IsLinks<T> {
  final List<T> _items = [];

  List<T> get items => List.unmodifiable(_items);
  int get length => _items.length;
  bool get isEmpty => _items.isEmpty;

  void add(T item) => _items.add(item);
  void remove(T item) => _items.remove(item);
  void clear() => _items.clear();
}

// ── Federation ──────────────────────────────────────────────────

/// Marks a collection as shareable across federated peers.
///
/// ```dart
/// @Shareable(status: 'read_write')
/// @collection
/// class SharedNotes { ... }
/// ```
const shareable = Shareable();

class Shareable {
  /// Sharing status: 'read_only', 'read_write', 'full'
  final String status;
  const Shareable({this.status = 'read_write'});
}

// ── Views ────────────────────────────────────────────────────────

/// Marks a class as a read-only view that merges data from multiple collections.
///
/// Views are read-only — no create/save/delete. The generated ViewDao
/// fetches from each source and merges results.
///
/// ```dart
/// @NodeDBView(sources: [
///   ViewSource(collection: 'users'),
///   ViewSource(collection: 'products', database: 'warehouse'),
/// ])
/// class UserProductView {
///   late String userName;
///   late String productName;
/// }
/// ```
class NodeDBView {
  final List<ViewSource> sources;
  final String strategy;
  final String? joinField;
  const NodeDBView({
    required this.sources,
    this.strategy = 'union',
    this.joinField,
  });
}

/// A source collection for a @NodeDBView.
class ViewSource {
  final String collection;
  final String schema;
  final String? database;
  const ViewSource({
    required this.collection,
    this.schema = 'public',
    this.database,
  });
}

// ── Preferences ──────────────────────────────────────────────────

/// Marks a class as a typed preference store.
///
/// Each field becomes a preference key with typed get/set accessors.
///
/// ```dart
/// @preferences
/// class AppPrefs {
///   String locale = 'en';
///   int fontSize = 14;
///   bool darkMode = false;
/// }
/// ```
const preferences = Preferences();

class Preferences {
  /// Store name override (defaults to snake_case of class name).
  final String? store;

  /// Default conflict resolution strategy.
  final String conflictResolution;

  const Preferences({
    this.store,
    this.conflictResolution = 'last_write_wins',
  });
}

// ── Enums ───────────────────────────────────────────────────────

/// Index type for `@Index` annotation.
enum IndexType {
  /// Standard B-tree value index.
  value,

  /// Hash index for equality-only lookups.
  hash,

  /// Full-text index for text search.
  fullText,
}
