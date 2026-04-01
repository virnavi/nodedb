import '../engine/nosql_engine.dart';
import '../engine/provenance_engine.dart';
import '../model/document.dart';
import '../model/provenance_envelope.dart';
import 'filter_query.dart';
import 'query_result.dart';

/// Base class for typed collection access.
///
/// Provides CRUD operations backed by [NoSqlEngine], with type-safe
/// serialization via [_fromMap] and [_toMap] callbacks.
///
/// Generated DAOs extend this class with typed filter extensions.
class CollectionAccessor<T> {
  final String collectionName;
  final NoSqlEngine _engine;
  final T Function(Map<String, dynamic>) _fromMap;
  final Map<String, dynamic> Function(T) _toMap;

  CollectionAccessor({
    required this.collectionName,
    required NoSqlEngine engine,
    required T Function(Map<String, dynamic>) fromMap,
    required Map<String, dynamic> Function(T) toMap,
  })  : _engine = engine,
        _fromMap = fromMap,
        _toMap = toMap;

  /// Create a new filter query for this collection.
  FilterQuery<T> filter() => FilterQuery<T>();

  /// Find a document by ID.
  T? findById(int id) {
    final doc = _engine.get(collectionName, id);
    if (doc == null) return null;
    return _fromMap(doc.data);
  }

  /// Find all documents, optionally filtered.
  List<T> findAll([FilterQuery<T>? query]) {
    final params = query?.build() ?? {};
    final docs = _engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) => _fromMap(doc.data)).toList();
  }

  /// Find the first document matching a filter.
  T? findFirst(FilterQuery<T> query) {
    final results = findAll(query..limit(1));
    return results.isEmpty ? null : results.first;
  }

  /// Count documents, optionally filtered.
  int count([FilterQuery<T>? query]) {
    if (query == null) return _engine.count(collectionName);
    // Count via findAll (Rust-side count with filter not exposed yet)
    return findAll(query).length;
  }

  /// Insert or update a document.
  void save(T item, {int? id}) {
    _engine.writeTxn([
      WriteOp.put(collectionName, data: _toMap(item), id: id),
    ]);
  }

  /// Insert or update multiple documents.
  void saveAll(List<T> items) {
    _engine.writeTxn(
      items.map((item) => WriteOp.put(collectionName, data: _toMap(item))).toList(),
    );
  }

  /// Delete a document by ID.
  void deleteById(int id) {
    _engine.writeTxn([WriteOp.delete(collectionName, id: id)]);
  }

  /// Delete multiple documents by ID.
  void deleteAllById(List<int> ids) {
    _engine.writeTxn(
      ids.map((id) => WriteOp.delete(collectionName, id: id)).toList(),
    );
  }

  /// Find all documents with their provenance envelopes attached.
  ///
  /// Returns [WithProvenance] wrappers. If [provenanceEngine] is null,
  /// all results will have `provenance: null`.
  List<WithProvenance<T>> findAllWithProvenance({
    FilterQuery<T>? query,
    ProvenanceEngine? provenanceEngine,
  }) {
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
      if (provenanceEngine != null) {
        final envelopes =
            provenanceEngine.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromMap(doc.data), envelope);
    }).toList();
  }
}
