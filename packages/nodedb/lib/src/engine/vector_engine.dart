import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/vector_record.dart';
import '../model/search_result.dart';
import '../util/msgpack.dart';

/// Configuration for opening a vector engine.
class VectorOpenConfig {
  final String path;
  final int dimension;
  final String metric;
  final int maxElements;

  const VectorOpenConfig({
    required this.path,
    required this.dimension,
    this.metric = 'cosine',
    this.maxElements = 100000,
  });
}

/// Typed wrapper for the NodeDB Vector engine.
class VectorEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  VectorEngine._(this._handle, this._bindings);

  /// Attach to an existing vector engine handle (for multi-isolate use).
  factory VectorEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return VectorEngine._(handle, bindings);
  }

  static VectorEngine open(NodeDbBindings bindings, VectorOpenConfig config) {
    final configBytes = msgpackEncode({
      'path': config.path,
      'dimension': config.dimension,
      'metric': config.metric,
      'max_elements': config.maxElements,
    });
    final handle = openRaw(bindings, bindings.vectorOpen, configBytes);
    return VectorEngine._(handle, bindings);
  }

  int get handle => _handle;

  VectorRecord insert(List<double> vector, {Map<String, dynamic>? metadata}) {
    final resp = _execute({
      'action': 'insert',
      'vector': vector,
      if (metadata != null) 'metadata': metadata,
    });
    return VectorRecord.fromMsgpack(resp);
  }

  VectorRecord? get(int id) {
    try {
      final resp = _execute({'action': 'get', 'id': id});
      if (resp == null) return null;
      // FFI returns {"record": ..., "vector": [...]} map
      if (resp is Map) {
        final record = resp['record'];
        if (record == null) return null;
        return VectorRecord.fromMsgpack(record);
      }
      return VectorRecord.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  void delete(int id) {
    _execute({'action': 'delete', 'id': id});
  }

  void updateMetadata(int id, Map<String, dynamic> metadata) {
    _execute({'action': 'update_metadata', 'id': id, 'metadata': metadata});
  }

  List<SearchResult> search(
    List<double> query, {
    int k = 10,
    int efSearch = 64,
  }) {
    final resp = _execute({
      'action': 'search',
      'query': query,
      'k': k,
      'ef_search': efSearch,
    });
    if (resp is! List) return [];
    return resp.map((r) => SearchResult.fromMsgpack(r)).toList();
  }

  int count() {
    final resp = _execute({'action': 'count'});
    return (resp is int) ? resp : 0;
  }

  void flush() {
    _execute({'action': 'flush'});
  }

  void close() {
    _bindings.vectorClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.vectorExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
