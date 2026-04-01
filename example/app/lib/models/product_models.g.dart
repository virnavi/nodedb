// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'product_models.dart';

// **************************************************************************
// CollectionGenerator
// **************************************************************************

// ── Schema ─────────────────────────────────────────────────────
const _productSchema = NodeDbSchema(
  name: 'products',
  schema: 'public',
  singleton: false,
  type: 'collection',
  fields: [
    SchemaField('id', 'string', indexed: true, unique: true),
    SchemaField('name', 'string'),
    SchemaField('description', 'string'),
    SchemaField('price', 'double'),
    SchemaField('category', 'string'),
    SchemaField('createdBy', 'string'),
    SchemaField('createdAt', 'datetime'),
    SchemaField('extras', 'jsonb'),
    SchemaField('productMetadata', 'jsonb'),
    SchemaField('tags', 'list'),
  ],
);

const _productSchemaJson = <String, dynamic>{
  'name': 'products',
  'schema': 'public',
  'type': 'collection',
  'required_fields': [
    'name',
    'description',
    'price',
    'category',
    'createdBy',
    'extras',
    'productMetadata',
    'tags',
  ],
  'field_types': {
    'id': 'string',
    'name': 'string',
    'description': 'string',
    'price': 'double',
    'category': 'string',
    'createdBy': 'string',
    'createdAt': 'datetime',
    'extras': 'jsonb',
    'productMetadata': 'jsonb',
    'tags': 'list',
  },
};

/// Fully qualified collection name: public.products
const productCollectionName = 'public.products';

// ── Serialization ──────────────────────────────────────────────
Product _$ProductFromMap(Map<String, dynamic> map) {
  return Product(
    id: map['id'] as String,
    name: map['name'] as String,
    description: map['description'] as String,
    price: (map['price'] as num).toDouble(),
    category: map['category'] as String,
    createdBy: map['createdBy'] as String,
    createdAt: map['createdAt'] != null
        ? DateTime.parse(map['createdAt'] as String)
        : null,
    extras: Map<String, dynamic>.from(map['extras'] as Map? ?? {}),
    productMetadata: _$ProductMetadataFromMap(
        Map<String, dynamic>.from(map['productMetadata'] as Map? ?? {})),
    tags: (map['tags'] as List? ?? []).cast<String>(),
  );
}

Map<String, dynamic> _$ProductToMap(Product instance) {
  return {
    'id': instance.id,
    'name': instance.name,
    'description': instance.description,
    'price': instance.price,
    'category': instance.category,
    'createdBy': instance.createdBy,
    'createdAt': instance.createdAt?.toIso8601String(),
    'extras': instance.extras,
    'productMetadata': _$ProductMetadataToMap(instance.productMetadata),
    'tags': instance.tags,
  };
}

// ── Filter Extensions ───────────────────────────────────────────
extension ProductFilterExtension on FilterQuery<Product> {
  FilterQuery<Product> idEqualTo(String value) => equalTo('id', value);
  FilterQuery<Product> idNotEqualTo(String value) => notEqualTo('id', value);
  FilterQuery<Product> idInList(List<String> values) => inList('id', values);
  FilterQuery<Product> idNotInList(List<String> values) =>
      notInList('id', values);
  FilterQuery<Product> idContains(String value) => contains('id', value);
  FilterQuery<Product> idStartsWith(String value) => startsWith('id', value);
  FilterQuery<Product> idEndsWith(String value) => endsWith('id', value);
  FilterQuery<Product> nameEqualTo(String value) => equalTo('name', value);
  FilterQuery<Product> nameNotEqualTo(String value) =>
      notEqualTo('name', value);
  FilterQuery<Product> nameInList(List<String> values) =>
      inList('name', values);
  FilterQuery<Product> nameNotInList(List<String> values) =>
      notInList('name', values);
  FilterQuery<Product> nameContains(String value) => contains('name', value);
  FilterQuery<Product> nameStartsWith(String value) =>
      startsWith('name', value);
  FilterQuery<Product> nameEndsWith(String value) => endsWith('name', value);
  FilterQuery<Product> descriptionEqualTo(String value) =>
      equalTo('description', value);
  FilterQuery<Product> descriptionNotEqualTo(String value) =>
      notEqualTo('description', value);
  FilterQuery<Product> descriptionInList(List<String> values) =>
      inList('description', values);
  FilterQuery<Product> descriptionNotInList(List<String> values) =>
      notInList('description', values);
  FilterQuery<Product> descriptionContains(String value) =>
      contains('description', value);
  FilterQuery<Product> descriptionStartsWith(String value) =>
      startsWith('description', value);
  FilterQuery<Product> descriptionEndsWith(String value) =>
      endsWith('description', value);
  FilterQuery<Product> priceEqualTo(double value) => equalTo('price', value);
  FilterQuery<Product> priceNotEqualTo(double value) =>
      notEqualTo('price', value);
  FilterQuery<Product> priceInList(List<double> values) =>
      inList('price', values);
  FilterQuery<Product> priceNotInList(List<double> values) =>
      notInList('price', values);
  FilterQuery<Product> priceGreaterThan(double value) =>
      greaterThan('price', value);
  FilterQuery<Product> priceGreaterThanOrEqual(double value) =>
      greaterThanOrEqual('price', value);
  FilterQuery<Product> priceLessThan(double value) => lessThan('price', value);
  FilterQuery<Product> priceLessThanOrEqual(double value) =>
      lessThanOrEqual('price', value);
  FilterQuery<Product> priceBetween(double low, double high) =>
      between('price', low, high);
  FilterQuery<Product> categoryEqualTo(String value) =>
      equalTo('category', value);
  FilterQuery<Product> categoryNotEqualTo(String value) =>
      notEqualTo('category', value);
  FilterQuery<Product> categoryInList(List<String> values) =>
      inList('category', values);
  FilterQuery<Product> categoryNotInList(List<String> values) =>
      notInList('category', values);
  FilterQuery<Product> categoryContains(String value) =>
      contains('category', value);
  FilterQuery<Product> categoryStartsWith(String value) =>
      startsWith('category', value);
  FilterQuery<Product> categoryEndsWith(String value) =>
      endsWith('category', value);
  FilterQuery<Product> createdByEqualTo(String value) =>
      equalTo('createdBy', value);
  FilterQuery<Product> createdByNotEqualTo(String value) =>
      notEqualTo('createdBy', value);
  FilterQuery<Product> createdByInList(List<String> values) =>
      inList('createdBy', values);
  FilterQuery<Product> createdByNotInList(List<String> values) =>
      notInList('createdBy', values);
  FilterQuery<Product> createdByContains(String value) =>
      contains('createdBy', value);
  FilterQuery<Product> createdByStartsWith(String value) =>
      startsWith('createdBy', value);
  FilterQuery<Product> createdByEndsWith(String value) =>
      endsWith('createdBy', value);
  FilterQuery<Product> createdAtEqualTo(DateTime value) =>
      equalTo('createdAt', value);
  FilterQuery<Product> createdAtNotEqualTo(DateTime value) =>
      notEqualTo('createdAt', value);
  FilterQuery<Product> createdAtInList(List<DateTime> values) =>
      inList('createdAt', values);
  FilterQuery<Product> createdAtNotInList(List<DateTime> values) =>
      notInList('createdAt', values);
  FilterQuery<Product> createdAtIsNull() => isNull('createdAt');
  FilterQuery<Product> createdAtIsNotNull() => isNotNull('createdAt');
  FilterQuery<Product> createdAtGreaterThan(DateTime value) =>
      greaterThan('createdAt', value);
  FilterQuery<Product> createdAtGreaterThanOrEqual(DateTime value) =>
      greaterThanOrEqual('createdAt', value);
  FilterQuery<Product> createdAtLessThan(DateTime value) =>
      lessThan('createdAt', value);
  FilterQuery<Product> createdAtLessThanOrEqual(DateTime value) =>
      lessThanOrEqual('createdAt', value);
  FilterQuery<Product> createdAtBetween(DateTime low, DateTime high) =>
      between('createdAt', low, high);
  FilterQuery<Product> extrasPathEquals(String path, dynamic value) =>
      jsonPathEquals('extras', path, value);
  FilterQuery<Product> extrasHasKey(String path) => jsonHasKey('extras', path);
  FilterQuery<Product> extrasContains(Map<String, dynamic> value) =>
      jsonContains('extras', value);
  FilterQuery<Product> productMetadataPathEquals(String path, dynamic value) =>
      jsonPathEquals('productMetadata', path, value);
  FilterQuery<Product> productMetadataHasKey(String path) =>
      jsonHasKey('productMetadata', path);
  FilterQuery<Product> productMetadataContains(Map<String, dynamic> value) =>
      jsonContains('productMetadata', value);
  FilterQuery<Product> tagsContains(String value) =>
      arrayContains('tags', value);
  FilterQuery<Product> tagsOverlaps(List<String> values) =>
      arrayOverlap('tags', values);
  FilterQuery<Product> sortById({bool desc = false}) =>
      sortBy('id', desc: desc);
  FilterQuery<Product> sortByName({bool desc = false}) =>
      sortBy('name', desc: desc);
  FilterQuery<Product> sortByDescription({bool desc = false}) =>
      sortBy('description', desc: desc);
  FilterQuery<Product> sortByPrice({bool desc = false}) =>
      sortBy('price', desc: desc);
  FilterQuery<Product> sortByCategory({bool desc = false}) =>
      sortBy('category', desc: desc);
  FilterQuery<Product> sortByCreatedBy({bool desc = false}) =>
      sortBy('createdBy', desc: desc);
  FilterQuery<Product> sortByCreatedAt({bool desc = false}) =>
      sortBy('createdAt', desc: desc);
}

// ── DAO ────────────────────────────────────────────────────────
abstract class ProductDaoBase {
  NoSqlEngine get _engine;
  ProvenanceEngine? get _provenanceEngine => null;
  CollectionNotifier? get _notifier => null;
  String? get _databaseName => null;

  String get collectionName => 'products';
  static const schemaName = 'public';
  String get qualifiedName {
    final db = _databaseName;
    if (db != null && db.isNotEmpty) return '$db.$schemaName.$collectionName';
    return '$schemaName.$collectionName';
  }

  Product _fromDocument(Document doc) => _$ProductFromMap(doc.data);
  Map<String, dynamic> _toMap(Product item) => _$ProductToMap(item);

  Document? _findDocumentById(String id) {
    final docs = _engine.findAll(
      collectionName,
      filter: {
        'Condition': {
          'EqualTo': {'field': 'id', 'value': id}
        }
      },
      limit: 1,
    );
    return docs.isEmpty ? null : docs.first;
  }

  Product? findById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create(Product item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache(Product item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map, cache: cache)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<Product> items) {
    _engine.writeTxn(
      items.map((item) {
        final map = _toMap(item);
        if (map['id'] == null ||
            (map['id'] is String && (map['id'] as String).isEmpty)) {
          map['id'] = generateNodeDbId();
        }
        return WriteOp.put(collectionName, data: map);
      }).toList(),
    );
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save(Product item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache(Product item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id, cache: cache),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<Product> items) {
    final ops = <WriteOp>[];
    for (final item in items) {
      final map = _toMap(item);
      if (map['id'] == null ||
          (map['id'] is String && (map['id'] as String).isEmpty)) {
        map['id'] = generateNodeDbId();
      }
      final existing = _findDocumentById(map['id'] as String);
      ops.add(WriteOp.put(collectionName, data: map, id: existing?.id));
    }
    _engine.writeTxn(ops);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  Product? updateById(String id, Product Function(Product current) modifier) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    final current = _fromDocument(doc);
    final updated = modifier(current);
    final map = _toMap(updated);
    map['id'] = id;
    _engine.writeTxn([WriteOp.put(collectionName, data: map, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return false;
    _engine.writeTxn([WriteOp.delete(collectionName, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<String> ids) {
    final ops = <WriteOp>[];
    for (final id in ids) {
      final doc = _findDocumentById(id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
  }

  int deleteWhere(FilterQuery<Product> Function(FilterQuery<Product>) filter) {
    final items = findWhere(filter);
    final ops = <WriteOp>[];
    for (final item in items) {
      final doc = _findDocumentById(item.id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
    return ops.length;
  }

  WithProvenance<Product>? findByIdWithProvenance(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (_provenanceEngine != null) {
      final envelopes = _provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<Product>> watchAll(
      {FilterQuery<Product>? query, bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findAll(query));
    return _notifier!.watch<List<Product>>(collectionName, () => findAll(query),
        fireImmediately: fireImmediately);
  }

  Stream<Product?> watchById(String id, {bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findById(id));
    return _notifier!.watch<Product?>(collectionName, () => findById(id),
        fireImmediately: fireImmediately);
  }

  Stream<List<Product>> watchWhere(
    FilterQuery<Product> Function(FilterQuery<Product>) filter, {
    bool fireImmediately = true,
  }) =>
      watchAll(
          query: filter(FilterQuery<Product>()),
          fireImmediately: fireImmediately);

  List<Product> findAll([FilterQuery<Product>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map(_fromDocument).toList();
  }

  Product? findFirst(
      FilterQuery<Product> Function(FilterQuery<Product>) filter) {
    final query = filter(FilterQuery<Product>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<Product> findWhere(
      FilterQuery<Product> Function(FilterQuery<Product>) filter) {
    return findAll(filter(FilterQuery<Product>()));
  }

  int count() => _engine.count(collectionName);

  int countWhere(FilterQuery<Product> Function(FilterQuery<Product>) filter) {
    return findWhere(filter).length;
  }

  bool exists(FilterQuery<Product> Function(FilterQuery<Product>) filter) {
    return findFirst(filter) != null;
  }

  List<Product> findPage({required int limit, int offset = 0}) {
    return findAll(FilterQuery<Product>()
      ..offset(offset)
      ..limit(limit));
  }

  List<Product> findPageWhere(
    FilterQuery<Product> Function(FilterQuery<Product>) filter, {
    required int limit,
    int offset = 0,
  }) {
    final query = filter(FilterQuery<Product>())
      ..offset(offset)
      ..limit(limit);
    return findAll(query);
  }

  List<WithProvenance<Product>> findAllWithProvenance(
      [FilterQuery<Product>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (_provenanceEngine != null) {
        final envelopes =
            _provenanceEngine!.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromDocument(doc), envelope);
    }).toList();
  }

  List<WithProvenance<Product>> findWhereWithProvenance(
    FilterQuery<Product> Function(FilterQuery<Product>) filter,
  ) {
    return findAllWithProvenance(filter(FilterQuery<Product>()));
  }

  /// Sweep expired cached records in this collection.
  /// Returns the count of deleted records.
  int sweepExpired() => _engine.sweepExpired(collectionName);
}

// ── Concrete DAO ───────────────────────────────────────────────
class ProductDao extends ProductDaoBase {
  @override
  final NoSqlEngine _engine;
  @override
  final ProvenanceEngine? _provenanceEngine;
  @override
  final CollectionNotifier? _notifier;
  @override
  final String? _databaseName;
  ProductDao(this._engine,
      [this._provenanceEngine, this._notifier, this._databaseName]);
  // Add custom query methods here
}

// ── Schema ─────────────────────────────────────────────────────
const _categorySchema = NodeDbSchema(
  name: 'categories',
  schema: 'public',
  singleton: false,
  type: 'collection',
  fields: [
    SchemaField('id', 'string', indexed: true, unique: true),
    SchemaField('name', 'string', indexed: true, unique: true),
  ],
);

const _categorySchemaJson = <String, dynamic>{
  'name': 'categories',
  'schema': 'public',
  'type': 'collection',
  'required_fields': [
    'name',
  ],
  'field_types': {
    'id': 'string',
    'name': 'string',
  },
};

/// Fully qualified collection name: public.categories
const categoryCollectionName = 'public.categories';

// ── Serialization ──────────────────────────────────────────────
Category _$CategoryFromMap(Map<String, dynamic> map) {
  return Category(
    id: map['id'] as String,
    name: map['name'] as String,
  );
}

Map<String, dynamic> _$CategoryToMap(Category instance) {
  return {
    'id': instance.id,
    'name': instance.name,
  };
}

// ── Filter Extensions ───────────────────────────────────────────
extension CategoryFilterExtension on FilterQuery<Category> {
  FilterQuery<Category> idEqualTo(String value) => equalTo('id', value);
  FilterQuery<Category> idNotEqualTo(String value) => notEqualTo('id', value);
  FilterQuery<Category> idInList(List<String> values) => inList('id', values);
  FilterQuery<Category> idNotInList(List<String> values) =>
      notInList('id', values);
  FilterQuery<Category> idContains(String value) => contains('id', value);
  FilterQuery<Category> idStartsWith(String value) => startsWith('id', value);
  FilterQuery<Category> idEndsWith(String value) => endsWith('id', value);
  FilterQuery<Category> nameEqualTo(String value) => equalTo('name', value);
  FilterQuery<Category> nameNotEqualTo(String value) =>
      notEqualTo('name', value);
  FilterQuery<Category> nameInList(List<String> values) =>
      inList('name', values);
  FilterQuery<Category> nameNotInList(List<String> values) =>
      notInList('name', values);
  FilterQuery<Category> nameContains(String value) => contains('name', value);
  FilterQuery<Category> nameStartsWith(String value) =>
      startsWith('name', value);
  FilterQuery<Category> nameEndsWith(String value) => endsWith('name', value);
  FilterQuery<Category> sortById({bool desc = false}) =>
      sortBy('id', desc: desc);
  FilterQuery<Category> sortByName({bool desc = false}) =>
      sortBy('name', desc: desc);
}

// ── DAO ────────────────────────────────────────────────────────
abstract class CategoryDaoBase {
  NoSqlEngine get _engine;
  ProvenanceEngine? get _provenanceEngine => null;
  CollectionNotifier? get _notifier => null;
  String? get _databaseName => null;

  String get collectionName => 'categories';
  static const schemaName = 'public';
  String get qualifiedName {
    final db = _databaseName;
    if (db != null && db.isNotEmpty) return '$db.$schemaName.$collectionName';
    return '$schemaName.$collectionName';
  }

  Category _fromDocument(Document doc) => _$CategoryFromMap(doc.data);
  Map<String, dynamic> _toMap(Category item) => _$CategoryToMap(item);

  Document? _findDocumentById(String id) {
    final docs = _engine.findAll(
      collectionName,
      filter: {
        'Condition': {
          'EqualTo': {'field': 'id', 'value': id}
        }
      },
      limit: 1,
    );
    return docs.isEmpty ? null : docs.first;
  }

  Category? findById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create(Category item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache(Category item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map, cache: cache)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<Category> items) {
    _engine.writeTxn(
      items.map((item) {
        final map = _toMap(item);
        if (map['id'] == null ||
            (map['id'] is String && (map['id'] as String).isEmpty)) {
          map['id'] = generateNodeDbId();
        }
        return WriteOp.put(collectionName, data: map);
      }).toList(),
    );
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save(Category item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache(Category item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id, cache: cache),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<Category> items) {
    final ops = <WriteOp>[];
    for (final item in items) {
      final map = _toMap(item);
      if (map['id'] == null ||
          (map['id'] is String && (map['id'] as String).isEmpty)) {
        map['id'] = generateNodeDbId();
      }
      final existing = _findDocumentById(map['id'] as String);
      ops.add(WriteOp.put(collectionName, data: map, id: existing?.id));
    }
    _engine.writeTxn(ops);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  Category? updateById(
      String id, Category Function(Category current) modifier) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    final current = _fromDocument(doc);
    final updated = modifier(current);
    final map = _toMap(updated);
    map['id'] = id;
    _engine.writeTxn([WriteOp.put(collectionName, data: map, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return false;
    _engine.writeTxn([WriteOp.delete(collectionName, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<String> ids) {
    final ops = <WriteOp>[];
    for (final id in ids) {
      final doc = _findDocumentById(id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
  }

  int deleteWhere(
      FilterQuery<Category> Function(FilterQuery<Category>) filter) {
    final items = findWhere(filter);
    final ops = <WriteOp>[];
    for (final item in items) {
      final doc = _findDocumentById(item.id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
    return ops.length;
  }

  WithProvenance<Category>? findByIdWithProvenance(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (_provenanceEngine != null) {
      final envelopes = _provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<Category>> watchAll(
      {FilterQuery<Category>? query, bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findAll(query));
    return _notifier!.watch<List<Category>>(
        collectionName, () => findAll(query),
        fireImmediately: fireImmediately);
  }

  Stream<Category?> watchById(String id, {bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findById(id));
    return _notifier!.watch<Category?>(collectionName, () => findById(id),
        fireImmediately: fireImmediately);
  }

  Stream<List<Category>> watchWhere(
    FilterQuery<Category> Function(FilterQuery<Category>) filter, {
    bool fireImmediately = true,
  }) =>
      watchAll(
          query: filter(FilterQuery<Category>()),
          fireImmediately: fireImmediately);

  List<Category> findAll([FilterQuery<Category>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map(_fromDocument).toList();
  }

  Category? findFirst(
      FilterQuery<Category> Function(FilterQuery<Category>) filter) {
    final query = filter(FilterQuery<Category>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<Category> findWhere(
      FilterQuery<Category> Function(FilterQuery<Category>) filter) {
    return findAll(filter(FilterQuery<Category>()));
  }

  int count() => _engine.count(collectionName);

  int countWhere(FilterQuery<Category> Function(FilterQuery<Category>) filter) {
    return findWhere(filter).length;
  }

  bool exists(FilterQuery<Category> Function(FilterQuery<Category>) filter) {
    return findFirst(filter) != null;
  }

  List<Category> findPage({required int limit, int offset = 0}) {
    return findAll(FilterQuery<Category>()
      ..offset(offset)
      ..limit(limit));
  }

  List<Category> findPageWhere(
    FilterQuery<Category> Function(FilterQuery<Category>) filter, {
    required int limit,
    int offset = 0,
  }) {
    final query = filter(FilterQuery<Category>())
      ..offset(offset)
      ..limit(limit);
    return findAll(query);
  }

  List<WithProvenance<Category>> findAllWithProvenance(
      [FilterQuery<Category>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (_provenanceEngine != null) {
        final envelopes =
            _provenanceEngine!.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromDocument(doc), envelope);
    }).toList();
  }

  List<WithProvenance<Category>> findWhereWithProvenance(
    FilterQuery<Category> Function(FilterQuery<Category>) filter,
  ) {
    return findAllWithProvenance(filter(FilterQuery<Category>()));
  }

  /// Sweep expired cached records in this collection.
  /// Returns the count of deleted records.
  int sweepExpired() => _engine.sweepExpired(collectionName);
}

// ── Concrete DAO ───────────────────────────────────────────────
class CategoryDao extends CategoryDaoBase {
  @override
  final NoSqlEngine _engine;
  @override
  final ProvenanceEngine? _provenanceEngine;
  @override
  final CollectionNotifier? _notifier;
  @override
  final String? _databaseName;
  CategoryDao(this._engine,
      [this._provenanceEngine, this._notifier, this._databaseName]);
  // Add custom query methods here
}

// ── Schema ─────────────────────────────────────────────────────
const _orderSchema = NodeDbSchema(
  name: 'orders',
  schema: 'public',
  singleton: false,
  type: 'collection',
  fields: [
    SchemaField('id', 'string', indexed: true, unique: true),
    SchemaField('productId', 'string'),
    SchemaField('buyerId', 'string'),
    SchemaField('status', 'string'),
    SchemaField('createdAt', 'datetime'),
  ],
);

const _orderSchemaJson = <String, dynamic>{
  'name': 'orders',
  'schema': 'public',
  'type': 'collection',
  'required_fields': [
    'productId',
    'buyerId',
    'status',
  ],
  'field_types': {
    'id': 'string',
    'productId': 'string',
    'buyerId': 'string',
    'status': 'string',
    'createdAt': 'datetime',
  },
};

/// Fully qualified collection name: public.orders
const orderCollectionName = 'public.orders';

// ── Serialization ──────────────────────────────────────────────
Order _$OrderFromMap(Map<String, dynamic> map) {
  return Order(
    id: map['id'] as String,
    productId: map['productId'] as String,
    buyerId: map['buyerId'] as String,
    status: map['status'] as String,
    createdAt: map['createdAt'] != null
        ? DateTime.parse(map['createdAt'] as String)
        : null,
  );
}

Map<String, dynamic> _$OrderToMap(Order instance) {
  return {
    'id': instance.id,
    'productId': instance.productId,
    'buyerId': instance.buyerId,
    'status': instance.status,
    'createdAt': instance.createdAt?.toIso8601String(),
  };
}

// ── Filter Extensions ───────────────────────────────────────────
extension OrderFilterExtension on FilterQuery<Order> {
  FilterQuery<Order> idEqualTo(String value) => equalTo('id', value);
  FilterQuery<Order> idNotEqualTo(String value) => notEqualTo('id', value);
  FilterQuery<Order> idInList(List<String> values) => inList('id', values);
  FilterQuery<Order> idNotInList(List<String> values) =>
      notInList('id', values);
  FilterQuery<Order> idContains(String value) => contains('id', value);
  FilterQuery<Order> idStartsWith(String value) => startsWith('id', value);
  FilterQuery<Order> idEndsWith(String value) => endsWith('id', value);
  FilterQuery<Order> productIdEqualTo(String value) =>
      equalTo('productId', value);
  FilterQuery<Order> productIdNotEqualTo(String value) =>
      notEqualTo('productId', value);
  FilterQuery<Order> productIdInList(List<String> values) =>
      inList('productId', values);
  FilterQuery<Order> productIdNotInList(List<String> values) =>
      notInList('productId', values);
  FilterQuery<Order> productIdContains(String value) =>
      contains('productId', value);
  FilterQuery<Order> productIdStartsWith(String value) =>
      startsWith('productId', value);
  FilterQuery<Order> productIdEndsWith(String value) =>
      endsWith('productId', value);
  FilterQuery<Order> buyerIdEqualTo(String value) => equalTo('buyerId', value);
  FilterQuery<Order> buyerIdNotEqualTo(String value) =>
      notEqualTo('buyerId', value);
  FilterQuery<Order> buyerIdInList(List<String> values) =>
      inList('buyerId', values);
  FilterQuery<Order> buyerIdNotInList(List<String> values) =>
      notInList('buyerId', values);
  FilterQuery<Order> buyerIdContains(String value) =>
      contains('buyerId', value);
  FilterQuery<Order> buyerIdStartsWith(String value) =>
      startsWith('buyerId', value);
  FilterQuery<Order> buyerIdEndsWith(String value) =>
      endsWith('buyerId', value);
  FilterQuery<Order> statusEqualTo(String value) => equalTo('status', value);
  FilterQuery<Order> statusNotEqualTo(String value) =>
      notEqualTo('status', value);
  FilterQuery<Order> statusInList(List<String> values) =>
      inList('status', values);
  FilterQuery<Order> statusNotInList(List<String> values) =>
      notInList('status', values);
  FilterQuery<Order> statusContains(String value) => contains('status', value);
  FilterQuery<Order> statusStartsWith(String value) =>
      startsWith('status', value);
  FilterQuery<Order> statusEndsWith(String value) => endsWith('status', value);
  FilterQuery<Order> createdAtEqualTo(DateTime value) =>
      equalTo('createdAt', value);
  FilterQuery<Order> createdAtNotEqualTo(DateTime value) =>
      notEqualTo('createdAt', value);
  FilterQuery<Order> createdAtInList(List<DateTime> values) =>
      inList('createdAt', values);
  FilterQuery<Order> createdAtNotInList(List<DateTime> values) =>
      notInList('createdAt', values);
  FilterQuery<Order> createdAtIsNull() => isNull('createdAt');
  FilterQuery<Order> createdAtIsNotNull() => isNotNull('createdAt');
  FilterQuery<Order> createdAtGreaterThan(DateTime value) =>
      greaterThan('createdAt', value);
  FilterQuery<Order> createdAtGreaterThanOrEqual(DateTime value) =>
      greaterThanOrEqual('createdAt', value);
  FilterQuery<Order> createdAtLessThan(DateTime value) =>
      lessThan('createdAt', value);
  FilterQuery<Order> createdAtLessThanOrEqual(DateTime value) =>
      lessThanOrEqual('createdAt', value);
  FilterQuery<Order> createdAtBetween(DateTime low, DateTime high) =>
      between('createdAt', low, high);
  FilterQuery<Order> sortById({bool desc = false}) => sortBy('id', desc: desc);
  FilterQuery<Order> sortByProductId({bool desc = false}) =>
      sortBy('productId', desc: desc);
  FilterQuery<Order> sortByBuyerId({bool desc = false}) =>
      sortBy('buyerId', desc: desc);
  FilterQuery<Order> sortByStatus({bool desc = false}) =>
      sortBy('status', desc: desc);
  FilterQuery<Order> sortByCreatedAt({bool desc = false}) =>
      sortBy('createdAt', desc: desc);
}

// ── DAO ────────────────────────────────────────────────────────
abstract class OrderDaoBase {
  NoSqlEngine get _engine;
  ProvenanceEngine? get _provenanceEngine => null;
  CollectionNotifier? get _notifier => null;
  String? get _databaseName => null;

  String get collectionName => 'orders';
  static const schemaName = 'public';
  String get qualifiedName {
    final db = _databaseName;
    if (db != null && db.isNotEmpty) return '$db.$schemaName.$collectionName';
    return '$schemaName.$collectionName';
  }

  Order _fromDocument(Document doc) => _$OrderFromMap(doc.data);
  Map<String, dynamic> _toMap(Order item) => _$OrderToMap(item);

  Document? _findDocumentById(String id) {
    final docs = _engine.findAll(
      collectionName,
      filter: {
        'Condition': {
          'EqualTo': {'field': 'id', 'value': id}
        }
      },
      limit: 1,
    );
    return docs.isEmpty ? null : docs.first;
  }

  Order? findById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create(Order item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache(Order item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map, cache: cache)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<Order> items) {
    _engine.writeTxn(
      items.map((item) {
        final map = _toMap(item);
        if (map['id'] == null ||
            (map['id'] is String && (map['id'] as String).isEmpty)) {
          map['id'] = generateNodeDbId();
        }
        return WriteOp.put(collectionName, data: map);
      }).toList(),
    );
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save(Order item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache(Order item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id, cache: cache),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<Order> items) {
    final ops = <WriteOp>[];
    for (final item in items) {
      final map = _toMap(item);
      if (map['id'] == null ||
          (map['id'] is String && (map['id'] as String).isEmpty)) {
        map['id'] = generateNodeDbId();
      }
      final existing = _findDocumentById(map['id'] as String);
      ops.add(WriteOp.put(collectionName, data: map, id: existing?.id));
    }
    _engine.writeTxn(ops);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  Order? updateById(String id, Order Function(Order current) modifier) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    final current = _fromDocument(doc);
    final updated = modifier(current);
    final map = _toMap(updated);
    map['id'] = id;
    _engine.writeTxn([WriteOp.put(collectionName, data: map, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return false;
    _engine.writeTxn([WriteOp.delete(collectionName, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<String> ids) {
    final ops = <WriteOp>[];
    for (final id in ids) {
      final doc = _findDocumentById(id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
  }

  int deleteWhere(FilterQuery<Order> Function(FilterQuery<Order>) filter) {
    final items = findWhere(filter);
    final ops = <WriteOp>[];
    for (final item in items) {
      final doc = _findDocumentById(item.id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
    return ops.length;
  }

  WithProvenance<Order>? findByIdWithProvenance(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (_provenanceEngine != null) {
      final envelopes = _provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<Order>> watchAll(
      {FilterQuery<Order>? query, bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findAll(query));
    return _notifier!.watch<List<Order>>(collectionName, () => findAll(query),
        fireImmediately: fireImmediately);
  }

  Stream<Order?> watchById(String id, {bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findById(id));
    return _notifier!.watch<Order?>(collectionName, () => findById(id),
        fireImmediately: fireImmediately);
  }

  Stream<List<Order>> watchWhere(
    FilterQuery<Order> Function(FilterQuery<Order>) filter, {
    bool fireImmediately = true,
  }) =>
      watchAll(
          query: filter(FilterQuery<Order>()),
          fireImmediately: fireImmediately);

  List<Order> findAll([FilterQuery<Order>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map(_fromDocument).toList();
  }

  Order? findFirst(FilterQuery<Order> Function(FilterQuery<Order>) filter) {
    final query = filter(FilterQuery<Order>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<Order> findWhere(
      FilterQuery<Order> Function(FilterQuery<Order>) filter) {
    return findAll(filter(FilterQuery<Order>()));
  }

  int count() => _engine.count(collectionName);

  int countWhere(FilterQuery<Order> Function(FilterQuery<Order>) filter) {
    return findWhere(filter).length;
  }

  bool exists(FilterQuery<Order> Function(FilterQuery<Order>) filter) {
    return findFirst(filter) != null;
  }

  List<Order> findPage({required int limit, int offset = 0}) {
    return findAll(FilterQuery<Order>()
      ..offset(offset)
      ..limit(limit));
  }

  List<Order> findPageWhere(
    FilterQuery<Order> Function(FilterQuery<Order>) filter, {
    required int limit,
    int offset = 0,
  }) {
    final query = filter(FilterQuery<Order>())
      ..offset(offset)
      ..limit(limit);
    return findAll(query);
  }

  List<WithProvenance<Order>> findAllWithProvenance(
      [FilterQuery<Order>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (_provenanceEngine != null) {
        final envelopes =
            _provenanceEngine!.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromDocument(doc), envelope);
    }).toList();
  }

  List<WithProvenance<Order>> findWhereWithProvenance(
    FilterQuery<Order> Function(FilterQuery<Order>) filter,
  ) {
    return findAllWithProvenance(filter(FilterQuery<Order>()));
  }

  /// Sweep expired cached records in this collection.
  /// Returns the count of deleted records.
  int sweepExpired() => _engine.sweepExpired(collectionName);
}

// ── Concrete DAO ───────────────────────────────────────────────
class OrderDao extends OrderDaoBase {
  @override
  final NoSqlEngine _engine;
  @override
  final ProvenanceEngine? _provenanceEngine;
  @override
  final CollectionNotifier? _notifier;
  @override
  final String? _databaseName;
  OrderDao(this._engine,
      [this._provenanceEngine, this._notifier, this._databaseName]);
  // Add custom query methods here
}

// ── Schema ─────────────────────────────────────────────────────
const _searchResultSchema = NodeDbSchema(
  name: 'search_results',
  schema: 'public',
  singleton: false,
  type: 'collection',
  fields: [
    SchemaField('id', 'string', indexed: true, unique: true),
    SchemaField('query', 'string'),
    SchemaField('resultJson', 'string'),
    SchemaField('cachedAt', 'datetime'),
  ],
  trimmable: true,
  trimPolicy: 'default',
);

const _searchResultSchemaJson = <String, dynamic>{
  'name': 'search_results',
  'schema': 'public',
  'type': 'collection',
  'required_fields': [
    'query',
    'resultJson',
  ],
  'field_types': {
    'id': 'string',
    'query': 'string',
    'resultJson': 'string',
    'cachedAt': 'datetime',
  },
};

/// Fully qualified collection name: public.search_results
const searchResultCollectionName = 'public.search_results';

// ── Serialization ──────────────────────────────────────────────
SearchResult _$SearchResultFromMap(Map<String, dynamic> map) {
  return SearchResult(
    id: map['id'] as String,
    query: map['query'] as String,
    resultJson: map['resultJson'] as String,
    cachedAt: map['cachedAt'] != null
        ? DateTime.parse(map['cachedAt'] as String)
        : null,
  );
}

Map<String, dynamic> _$SearchResultToMap(SearchResult instance) {
  return {
    'id': instance.id,
    'query': instance.query,
    'resultJson': instance.resultJson,
    'cachedAt': instance.cachedAt?.toIso8601String(),
  };
}

// ── Filter Extensions ───────────────────────────────────────────
extension SearchResultFilterExtension on FilterQuery<SearchResult> {
  FilterQuery<SearchResult> idEqualTo(String value) => equalTo('id', value);
  FilterQuery<SearchResult> idNotEqualTo(String value) =>
      notEqualTo('id', value);
  FilterQuery<SearchResult> idInList(List<String> values) =>
      inList('id', values);
  FilterQuery<SearchResult> idNotInList(List<String> values) =>
      notInList('id', values);
  FilterQuery<SearchResult> idContains(String value) => contains('id', value);
  FilterQuery<SearchResult> idStartsWith(String value) =>
      startsWith('id', value);
  FilterQuery<SearchResult> idEndsWith(String value) => endsWith('id', value);
  FilterQuery<SearchResult> queryEqualTo(String value) =>
      equalTo('query', value);
  FilterQuery<SearchResult> queryNotEqualTo(String value) =>
      notEqualTo('query', value);
  FilterQuery<SearchResult> queryInList(List<String> values) =>
      inList('query', values);
  FilterQuery<SearchResult> queryNotInList(List<String> values) =>
      notInList('query', values);
  FilterQuery<SearchResult> queryContains(String value) =>
      contains('query', value);
  FilterQuery<SearchResult> queryStartsWith(String value) =>
      startsWith('query', value);
  FilterQuery<SearchResult> queryEndsWith(String value) =>
      endsWith('query', value);
  FilterQuery<SearchResult> resultJsonEqualTo(String value) =>
      equalTo('resultJson', value);
  FilterQuery<SearchResult> resultJsonNotEqualTo(String value) =>
      notEqualTo('resultJson', value);
  FilterQuery<SearchResult> resultJsonInList(List<String> values) =>
      inList('resultJson', values);
  FilterQuery<SearchResult> resultJsonNotInList(List<String> values) =>
      notInList('resultJson', values);
  FilterQuery<SearchResult> resultJsonContains(String value) =>
      contains('resultJson', value);
  FilterQuery<SearchResult> resultJsonStartsWith(String value) =>
      startsWith('resultJson', value);
  FilterQuery<SearchResult> resultJsonEndsWith(String value) =>
      endsWith('resultJson', value);
  FilterQuery<SearchResult> cachedAtEqualTo(DateTime value) =>
      equalTo('cachedAt', value);
  FilterQuery<SearchResult> cachedAtNotEqualTo(DateTime value) =>
      notEqualTo('cachedAt', value);
  FilterQuery<SearchResult> cachedAtInList(List<DateTime> values) =>
      inList('cachedAt', values);
  FilterQuery<SearchResult> cachedAtNotInList(List<DateTime> values) =>
      notInList('cachedAt', values);
  FilterQuery<SearchResult> cachedAtIsNull() => isNull('cachedAt');
  FilterQuery<SearchResult> cachedAtIsNotNull() => isNotNull('cachedAt');
  FilterQuery<SearchResult> cachedAtGreaterThan(DateTime value) =>
      greaterThan('cachedAt', value);
  FilterQuery<SearchResult> cachedAtGreaterThanOrEqual(DateTime value) =>
      greaterThanOrEqual('cachedAt', value);
  FilterQuery<SearchResult> cachedAtLessThan(DateTime value) =>
      lessThan('cachedAt', value);
  FilterQuery<SearchResult> cachedAtLessThanOrEqual(DateTime value) =>
      lessThanOrEqual('cachedAt', value);
  FilterQuery<SearchResult> cachedAtBetween(DateTime low, DateTime high) =>
      between('cachedAt', low, high);
  FilterQuery<SearchResult> sortById({bool desc = false}) =>
      sortBy('id', desc: desc);
  FilterQuery<SearchResult> sortByQuery({bool desc = false}) =>
      sortBy('query', desc: desc);
  FilterQuery<SearchResult> sortByResultJson({bool desc = false}) =>
      sortBy('resultJson', desc: desc);
  FilterQuery<SearchResult> sortByCachedAt({bool desc = false}) =>
      sortBy('cachedAt', desc: desc);
}

// ── DAO ────────────────────────────────────────────────────────
abstract class SearchResultDaoBase {
  NoSqlEngine get _engine;
  ProvenanceEngine? get _provenanceEngine => null;
  CollectionNotifier? get _notifier => null;
  String? get _databaseName => null;

  String get collectionName => 'search_results';
  static const schemaName = 'public';
  String get qualifiedName {
    final db = _databaseName;
    if (db != null && db.isNotEmpty) return '$db.$schemaName.$collectionName';
    return '$schemaName.$collectionName';
  }

  SearchResult _fromDocument(Document doc) => _$SearchResultFromMap(doc.data);
  Map<String, dynamic> _toMap(SearchResult item) => _$SearchResultToMap(item);

  Document? _findDocumentById(String id) {
    final docs = _engine.findAll(
      collectionName,
      filter: {
        'Condition': {
          'EqualTo': {'field': 'id', 'value': id}
        }
      },
      limit: 1,
    );
    return docs.isEmpty ? null : docs.first;
  }

  SearchResult? findById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create(SearchResult item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache(SearchResult item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    _engine.writeTxn([WriteOp.put(collectionName, data: map, cache: cache)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<SearchResult> items) {
    _engine.writeTxn(
      items.map((item) {
        final map = _toMap(item);
        if (map['id'] == null ||
            (map['id'] is String && (map['id'] as String).isEmpty)) {
          map['id'] = generateNodeDbId();
        }
        return WriteOp.put(collectionName, data: map);
      }).toList(),
    );
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save(SearchResult item) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache(SearchResult item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null ||
        (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    _engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id, cache: cache),
    ]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<SearchResult> items) {
    final ops = <WriteOp>[];
    for (final item in items) {
      final map = _toMap(item);
      if (map['id'] == null ||
          (map['id'] is String && (map['id'] as String).isEmpty)) {
        map['id'] = generateNodeDbId();
      }
      final existing = _findDocumentById(map['id'] as String);
      ops.add(WriteOp.put(collectionName, data: map, id: existing?.id));
    }
    _engine.writeTxn(ops);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  SearchResult? updateById(
      String id, SearchResult Function(SearchResult current) modifier) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    final current = _fromDocument(doc);
    final updated = modifier(current);
    final map = _toMap(updated);
    map['id'] = id;
    _engine.writeTxn([WriteOp.put(collectionName, data: map, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return false;
    _engine.writeTxn([WriteOp.delete(collectionName, id: doc.id)]);
    _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<String> ids) {
    final ops = <WriteOp>[];
    for (final id in ids) {
      final doc = _findDocumentById(id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
  }

  int deleteWhere(
      FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter) {
    final items = findWhere(filter);
    final ops = <WriteOp>[];
    for (final item in items) {
      final doc = _findDocumentById(item.id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      _engine.writeTxn(ops);
      _notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
    return ops.length;
  }

  WithProvenance<SearchResult>? findByIdWithProvenance(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (_provenanceEngine != null) {
      final envelopes = _provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<SearchResult>> watchAll(
      {FilterQuery<SearchResult>? query, bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findAll(query));
    return _notifier!.watch<List<SearchResult>>(
        collectionName, () => findAll(query),
        fireImmediately: fireImmediately);
  }

  Stream<SearchResult?> watchById(String id, {bool fireImmediately = true}) {
    if (_notifier == null) return Stream.value(findById(id));
    return _notifier!.watch<SearchResult?>(collectionName, () => findById(id),
        fireImmediately: fireImmediately);
  }

  Stream<List<SearchResult>> watchWhere(
    FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter, {
    bool fireImmediately = true,
  }) =>
      watchAll(
          query: filter(FilterQuery<SearchResult>()),
          fireImmediately: fireImmediately);

  List<SearchResult> findAll([FilterQuery<SearchResult>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map(_fromDocument).toList();
  }

  SearchResult? findFirst(
      FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter) {
    final query = filter(FilterQuery<SearchResult>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<SearchResult> findWhere(
      FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter) {
    return findAll(filter(FilterQuery<SearchResult>()));
  }

  int count() => _engine.count(collectionName);

  int countWhere(
      FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter) {
    return findWhere(filter).length;
  }

  bool exists(
      FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter) {
    return findFirst(filter) != null;
  }

  List<SearchResult> findPage({required int limit, int offset = 0}) {
    return findAll(FilterQuery<SearchResult>()
      ..offset(offset)
      ..limit(limit));
  }

  List<SearchResult> findPageWhere(
    FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter, {
    required int limit,
    int offset = 0,
  }) {
    final query = filter(FilterQuery<SearchResult>())
      ..offset(offset)
      ..limit(limit);
    return findAll(query);
  }

  List<WithProvenance<SearchResult>> findAllWithProvenance(
      [FilterQuery<SearchResult>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (_provenanceEngine != null) {
        final envelopes =
            _provenanceEngine!.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromDocument(doc), envelope);
    }).toList();
  }

  List<WithProvenance<SearchResult>> findWhereWithProvenance(
    FilterQuery<SearchResult> Function(FilterQuery<SearchResult>) filter,
  ) {
    return findAllWithProvenance(filter(FilterQuery<SearchResult>()));
  }

  /// Sweep expired cached records in this collection.
  /// Returns the count of deleted records.
  int sweepExpired() => _engine.sweepExpired(collectionName);

  bool get isTrimmable => true;

  TrimReport trim(TrimPolicy policy, {bool dryRun = false}) {
    return _engine.trim(collectionName, policy, dryRun: dryRun);
  }

  TrimRecommendation recommendTrim(TrimPolicy policy) {
    return _engine.recommendTrim(policy);
  }

  void setTrimPolicy(TrimPolicy policy) {
    _engine.trimConfigSet(collectionName, policy);
  }

  TrimPolicy? get effectiveTrimPolicy =>
      _engine.trimConfigEffective(collectionName);
}

// ── Concrete DAO ───────────────────────────────────────────────
class SearchResultDao extends SearchResultDaoBase {
  @override
  final NoSqlEngine _engine;
  @override
  final ProvenanceEngine? _provenanceEngine;
  @override
  final CollectionNotifier? _notifier;
  @override
  final String? _databaseName;
  SearchResultDao(this._engine,
      [this._provenanceEngine, this._notifier, this._databaseName]);
  // Add custom query methods here
}

// **************************************************************************
// ViewGenerator
// **************************************************************************

// ── Schema ─────────────────────────────────────────────────────
const _userProductViewSchema = NodeDbSchema(
  name: 'user_product_views',
  schema: 'public',
  singleton: false,
  type: 'view',
  fields: [
    SchemaField('name', 'string'),
    SchemaField('description', 'string'),
    SchemaField('price', 'double'),
  ],
);

const _userProductViewSchemaJson = <String, dynamic>{
  'name': 'user_product_views',
  'schema': 'public',
  'type': 'view',
  'required_fields': [
    'name',
    'description',
    'price',
  ],
  'field_types': {
    'name': 'string',
    'description': 'string',
    'price': 'double',
  },
};

/// Fully qualified collection name: public.user_product_views
const userProductViewCollectionName = 'public.user_product_views';

// ── Serialization ──────────────────────────────────────────────
UserProductView _$UserProductViewFromMap(Map<String, dynamic> map) {
  return UserProductView(
    name: map['name'] as String,
    description: map['description'] as String,
    price: (map['price'] as num).toDouble(),
  );
}

Map<String, dynamic> _$UserProductViewToMap(UserProductView instance) {
  return {
    'name': instance.name,
    'description': instance.description,
    'price': instance.price,
  };
}

// ── Filter Extensions ───────────────────────────────────────────
extension UserProductViewFilterExtension on FilterQuery<UserProductView> {
  FilterQuery<UserProductView> nameEqualTo(String value) =>
      equalTo('name', value);
  FilterQuery<UserProductView> nameNotEqualTo(String value) =>
      notEqualTo('name', value);
  FilterQuery<UserProductView> nameInList(List<String> values) =>
      inList('name', values);
  FilterQuery<UserProductView> nameNotInList(List<String> values) =>
      notInList('name', values);
  FilterQuery<UserProductView> nameContains(String value) =>
      contains('name', value);
  FilterQuery<UserProductView> nameStartsWith(String value) =>
      startsWith('name', value);
  FilterQuery<UserProductView> nameEndsWith(String value) =>
      endsWith('name', value);
  FilterQuery<UserProductView> descriptionEqualTo(String value) =>
      equalTo('description', value);
  FilterQuery<UserProductView> descriptionNotEqualTo(String value) =>
      notEqualTo('description', value);
  FilterQuery<UserProductView> descriptionInList(List<String> values) =>
      inList('description', values);
  FilterQuery<UserProductView> descriptionNotInList(List<String> values) =>
      notInList('description', values);
  FilterQuery<UserProductView> descriptionContains(String value) =>
      contains('description', value);
  FilterQuery<UserProductView> descriptionStartsWith(String value) =>
      startsWith('description', value);
  FilterQuery<UserProductView> descriptionEndsWith(String value) =>
      endsWith('description', value);
  FilterQuery<UserProductView> priceEqualTo(double value) =>
      equalTo('price', value);
  FilterQuery<UserProductView> priceNotEqualTo(double value) =>
      notEqualTo('price', value);
  FilterQuery<UserProductView> priceInList(List<double> values) =>
      inList('price', values);
  FilterQuery<UserProductView> priceNotInList(List<double> values) =>
      notInList('price', values);
  FilterQuery<UserProductView> priceGreaterThan(double value) =>
      greaterThan('price', value);
  FilterQuery<UserProductView> priceGreaterThanOrEqual(double value) =>
      greaterThanOrEqual('price', value);
  FilterQuery<UserProductView> priceLessThan(double value) =>
      lessThan('price', value);
  FilterQuery<UserProductView> priceLessThanOrEqual(double value) =>
      lessThanOrEqual('price', value);
  FilterQuery<UserProductView> priceBetween(double low, double high) =>
      between('price', low, high);
  FilterQuery<UserProductView> sortByName({bool desc = false}) =>
      sortBy('name', desc: desc);
  FilterQuery<UserProductView> sortByDescription({bool desc = false}) =>
      sortBy('description', desc: desc);
  FilterQuery<UserProductView> sortByPrice({bool desc = false}) =>
      sortBy('price', desc: desc);
}

// ── View DAO ───────────────────────────────────────────────────
abstract class UserProductViewViewDaoBase {
  NoSqlEngine get _engine;
  TransportEngine? get _transport => null;
  String? get _databaseName => null;

  static const schemaName = 'public';
  String get collectionName => 'user_product_views';
  String get qualifiedName {
    final db = _databaseName;
    if (db != null && db.isNotEmpty) return '$db.$schemaName.$collectionName';
    return '$schemaName.$collectionName';
  }

  static const sources = <Map<String, String>>[
    {'collection': 'users', 'schema': 'public', 'database': 'users'},
    {'collection': 'products', 'schema': 'public'},
  ];

  UserProductView _fromMap(Map<String, dynamic> map) =>
      _$UserProductViewFromMap(map);

  List<UserProductView> findAll([FilterQuery<UserProductView>? query]) {
    final params = query?.build() ?? {};
    final filter = params['filter'] as Map<String, dynamic>?;
    final sort = (params['sort'] as List?)?.cast<Map<String, dynamic>>();
    final offset = params['offset'] as int?;
    final limit = params['limit'] as int?;

    final results = <UserProductView>[];

    for (final source in sources) {
      final collection = source['collection']!;
      final database = source['database'];

      if (database != null && database.isNotEmpty && _transport != null) {
        // Remote source — query via mesh
        final remoteDocs = _transport!.meshQuery(
          database: database,
          queryType: 'find_all',
          queryData: {
            'collection': collection,
            if (filter != null) 'filter': filter,
            if (sort != null) 'sort': sort,
          },
        );
        results.addAll(remoteDocs.map((doc) => _fromMap(doc.data)));
      } else {
        // Local source
        final docs = _engine.findAll(
          collection,
          filter: filter,
          sort: sort,
        );
        results.addAll(docs.map((doc) => _fromMap(doc.data)));
      }
    }

    // Apply offset/limit after merge
    var merged = results;
    if (offset != null && offset > 0) {
      merged = merged.skip(offset).toList();
    }
    if (limit != null && limit > 0) {
      merged = merged.take(limit).toList();
    }

    return merged;
  }

  UserProductView? findFirst(
      FilterQuery<UserProductView> Function(FilterQuery<UserProductView>)
          filter) {
    final query = filter(FilterQuery<UserProductView>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<UserProductView> findWhere(
      FilterQuery<UserProductView> Function(FilterQuery<UserProductView>)
          filter) {
    return findAll(filter(FilterQuery<UserProductView>()));
  }

  int count() {
    var total = 0;
    for (final source in sources) {
      final collection = source['collection']!;
      final database = source['database'];
      if (database != null && database.isNotEmpty && _transport != null) {
        final result = _transport!.meshQuery(
          database: database,
          queryType: 'count',
          queryData: {'collection': collection},
        );
        total += (result is int ? result : (result as List).length);
      } else {
        total += _engine.count(collection);
      }
    }
    return total;
  }
}

// ── Concrete View DAO ──────────────────────────────────────────
class UserProductViewDao extends UserProductViewViewDaoBase {
  @override
  final NoSqlEngine _engine;
  @override
  final TransportEngine? _transport;
  @override
  final String? _databaseName;
  UserProductViewDao(this._engine, [this._transport, this._databaseName]);
}

// **************************************************************************
// JsonModelGenerator
// **************************************************************************

// ── Serialization ──────────────────────────────────────────────
ProductMetadata _$ProductMetadataFromMap(Map<String, dynamic> map) {
  return ProductMetadata(
    color: map['color'] as String,
    weight: (map['weight'] as num).toInt(),
    material: map['material'] as String?,
  );
}

Map<String, dynamic> _$ProductMetadataToMap(ProductMetadata instance) {
  return {
    'color': instance.color,
    'weight': instance.weight,
    'material': instance.material,
  };
}
