import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB AI Query engine.
class AiQueryEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  AiQueryEngine._(this._handle, this._bindings);

  /// Attach to an existing AI query engine handle (for multi-isolate use).
  factory AiQueryEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return AiQueryEngine._(handle, bindings);
  }

  static AiQueryEngine open(
    NodeDbBindings bindings, {
    required int nosqlHandle,
    required int provenanceHandle,
    List<String>? enabledCollections,
    double? minimumWriteConfidence,
    int? maxResultsPerQuery,
  }) {
    final configBytes = msgpackEncode({
      'nosql_handle': nosqlHandle,
      'provenance_handle': provenanceHandle,
      if (enabledCollections != null) 'enabled_collections': enabledCollections,
      if (minimumWriteConfidence != null) 'minimum_write_confidence': minimumWriteConfidence,
      if (maxResultsPerQuery != null) 'max_results_per_query': maxResultsPerQuery,
    });
    final handle = openRaw(bindings, bindings.aiQueryOpen, configBytes);
    return AiQueryEngine._(handle, bindings);
  }

  int get handle => _handle;

  dynamic processResults({
    required String collection,
    required List<Map<String, dynamic>> results,
    Map<String, dynamic>? schema,
  }) {
    return _execute({
      'action': 'process_results',
      'collection': collection,
      'results': results,
      if (schema != null) 'schema': schema,
    });
  }

  Map<String, dynamic> getConfig() {
    final resp = _execute({'action': 'get_config'});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  void close() {
    _bindings.aiQueryClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.aiQueryExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
