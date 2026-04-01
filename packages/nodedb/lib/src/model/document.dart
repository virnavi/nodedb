import '../util/msgpack.dart';
import 'cache_config.dart';

/// A NoSQL document stored in a NodeDB collection.
class Document {
  final int id;
  final String collection;
  final Map<String, dynamic> data;
  final DateTime createdAt;
  final DateTime updatedAt;

  const Document({
    required this.id,
    required this.collection,
    required this.data,
    required this.createdAt,
    required this.updatedAt,
  });

  /// Decode a Document from a MessagePack response.
  ///
  /// Handles both map-format (manually built in FFI) and positional-array
  /// format (from rmpv::ext::to_value).
  factory Document.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final collection = (decodeField(decoded, 'collection', 1) ?? '') as String;
    final rawData = decodeField(decoded, 'data', 2);
    final data = rawData is Map
        ? Map<String, dynamic>.from(rawData)
        : <String, dynamic>{};
    final createdAtStr = decodeField(decoded, 'created_at', 3);
    final updatedAtStr = decodeField(decoded, 'updated_at', 4);

    return Document(
      id: id,
      collection: collection,
      data: data,
      createdAt: _parseDateTime(createdAtStr),
      updatedAt: _parseDateTime(updatedAtStr),
    );
  }

  static DateTime _parseDateTime(dynamic value) {
    if (value is String) {
      return DateTime.tryParse(value) ?? DateTime.fromMillisecondsSinceEpoch(0);
    }
    return DateTime.fromMillisecondsSinceEpoch(0);
  }

  @override
  String toString() => 'Document(id: $id, collection: $collection, data: $data)';
}

/// A write operation for use in [NoSqlEngine.writeTxn].
class WriteOp {
  final String collection;
  final String action;
  final int? id;
  final Map<String, dynamic>? data;

  // Singleton/preference fields
  final String? store;
  final String? key;
  final dynamic value;
  final bool? shareable;
  final String? conflictResolution;
  final Map<String, dynamic>? defaults;
  final CacheConfig? cache;

  const WriteOp._({
    required this.collection,
    required this.action,
    this.id,
    this.data,
    this.store,
    this.key,
    this.value,
    this.shareable,
    this.conflictResolution,
    this.defaults,
    this.cache,
  });

  /// Create a put (insert/update) operation.
  ///
  /// Optionally pass a [cache] config to set a TTL on this record.
  factory WriteOp.put(String collection, {required Map<String, dynamic> data, int? id, CacheConfig? cache}) {
    return WriteOp._(collection: collection, action: 'put', data: data, id: id, cache: cache);
  }

  /// Create a delete operation.
  factory WriteOp.delete(String collection, {required int id}) {
    return WriteOp._(collection: collection, action: 'delete', id: id);
  }

  /// Create a singleton put operation.
  factory WriteOp.singletonPut(String collection, {required Map<String, dynamic> data}) {
    return WriteOp._(collection: collection, action: 'singleton_put', data: data);
  }

  /// Create a singleton create operation with default values.
  factory WriteOp.singletonCreate(
    String collection, {
    required Map<String, dynamic> defaults,
  }) {
    return WriteOp._(
      collection: collection,
      action: 'singleton_create',
      defaults: defaults,
    );
  }

  /// Create a singleton reset operation.
  factory WriteOp.singletonReset(String collection) {
    return WriteOp._(collection: collection, action: 'singleton_reset');
  }

  /// Create a preference set operation.
  factory WriteOp.prefSet(
    String store,
    String key,
    dynamic value, {
    bool shareable = false,
    String conflictResolution = 'last_write_wins',
  }) {
    return WriteOp._(
      collection: '_',
      action: 'pref_set',
      store: store,
      key: key,
      value: value,
      shareable: shareable,
      conflictResolution: conflictResolution,
    );
  }

  /// Create a preference remove operation.
  factory WriteOp.prefRemove(String store, String key) {
    return WriteOp._(collection: '_', action: 'pref_remove', store: store, key: key);
  }

  /// Serialize to a map for MessagePack encoding.
  Map<String, dynamic> toMap() {
    final map = <String, dynamic>{
      'collection': collection,
      'action': action,
    };
    if (id != null) map['id'] = id;
    if (data != null) map['data'] = data;
    if (store != null) map['store'] = store;
    if (key != null) map['key'] = key;
    if (value != null) map['value'] = value;
    if (shareable != null) map['shareable'] = shareable;
    if (conflictResolution != null) {
      map['conflict_resolution'] = conflictResolution;
    }
    if (defaults != null) map['defaults'] = defaults;
    if (cache != null) map['cache'] = cache!.toMap();
    return map;
  }
}
