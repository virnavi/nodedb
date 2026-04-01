import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB AI Provenance engine.
class AiProvenanceEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  AiProvenanceEngine._(this._handle, this._bindings);

  /// Attach to an existing AI provenance engine handle (for multi-isolate use).
  factory AiProvenanceEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return AiProvenanceEngine._(handle, bindings);
  }

  static AiProvenanceEngine open(
    NodeDbBindings bindings,
    int provenanceHandle,
  ) {
    final configBytes = msgpackEncode({
      'provenance_handle': provenanceHandle,
    });
    final handle =
        openRaw(bindings, bindings.aiProvenanceOpen, configBytes);
    return AiProvenanceEngine._(handle, bindings);
  }

  int get handle => _handle;

  dynamic applyAssessment({
    required int envelopeId,
    required double suggestedConfidence,
    String? sourceType,
    String? reasoning,
    Map<String, String>? tags,
  }) {
    return _execute({
      'action': 'apply_assessment',
      'envelope_id': envelopeId,
      'suggested_confidence': suggestedConfidence,
      if (sourceType != null) 'source_type': sourceType,
      if (reasoning != null) 'reasoning': reasoning,
      if (tags != null) 'tags': tags,
    });
  }

  dynamic applyConflictResolution({
    required int envelopeIdA,
    required int envelopeIdB,
    required double confidenceDeltaA,
    required double confidenceDeltaB,
    String preference = 'prefer_neither',
    String? reasoning,
  }) {
    return _execute({
      'action': 'apply_conflict_resolution',
      'envelope_id_a': envelopeIdA,
      'envelope_id_b': envelopeIdB,
      'confidence_delta_a': confidenceDeltaA,
      'confidence_delta_b': confidenceDeltaB,
      'preference': preference,
      if (reasoning != null) 'reasoning': reasoning,
    });
  }

  dynamic applyAnomalyFlags({
    required String collection,
    required List<Map<String, dynamic>> flags,
  }) {
    return _execute({
      'action': 'apply_anomaly_flags',
      'collection': collection,
      'flags': flags,
    });
  }

  dynamic applySourceClassification({
    required int envelopeId,
    required String sourceType,
    required double credibilityPrior,
    String? reasoning,
  }) {
    return _execute({
      'action': 'apply_source_classification',
      'envelope_id': envelopeId,
      'source_type': sourceType,
      'credibility_prior': credibilityPrior,
      if (reasoning != null) 'reasoning': reasoning,
    });
  }

  Map<String, dynamic> getConfig() {
    final resp = _execute({'action': 'get_config'});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  void close() {
    _bindings.aiProvenanceClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.aiProvenanceExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
