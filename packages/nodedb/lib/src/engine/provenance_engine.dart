import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/provenance_envelope.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB Provenance engine.
class ProvenanceEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  ProvenanceEngine._(this._handle, this._bindings);

  /// Attach to an existing provenance engine handle (for multi-isolate use).
  factory ProvenanceEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return ProvenanceEngine._(handle, bindings);
  }

  static ProvenanceEngine open(NodeDbBindings bindings, String path) {
    final handle =
        openRaw(bindings, bindings.provenanceOpen, buildConfig(path));
    return ProvenanceEngine._(handle, bindings);
  }

  int get handle => _handle;

  ProvenanceEnvelope attach({
    required String collection,
    required int recordId,
    required String sourceId,
    required String sourceType,
    required String contentHash,
    String? pkiSignature,
    String? pkiId,
    String? userId,
    bool? isSigned,
    int? hops,
    String? createdAtUtc,
    String? dataUpdatedAtUtc,
    String? localId,
    String? globalId,
  }) {
    final resp = _execute({
      'action': 'attach',
      'collection': collection,
      'record_id': recordId,
      'source_id': sourceId,
      'source_type': sourceType,
      'content_hash': contentHash,
      if (pkiSignature != null) 'pki_signature': pkiSignature,
      if (pkiId != null) 'pki_id': pkiId,
      if (userId != null) 'user_id': userId,
      if (isSigned != null) 'is_signed': isSigned,
      if (hops != null) 'hops': hops,
      if (createdAtUtc != null) 'created_at_utc': createdAtUtc,
      if (dataUpdatedAtUtc != null) 'data_updated_at_utc': dataUpdatedAtUtc,
      if (localId != null) 'local_id': localId,
      if (globalId != null) 'global_id': globalId,
    });
    return ProvenanceEnvelope.fromMsgpack(resp);
  }

  ProvenanceEnvelope? get(int id) {
    try {
      final resp = _execute({'action': 'get', 'id': id});
      if (resp == null) return null;
      return ProvenanceEnvelope.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  List<ProvenanceEnvelope> getForRecord(String collection, int recordId) {
    final resp = _execute({
      'action': 'get_for_record',
      'collection': collection,
      'record_id': recordId,
    });
    if (resp is! List) return [];
    return resp.map((e) => ProvenanceEnvelope.fromMsgpack(e)).toList();
  }

  ProvenanceEnvelope corroborate(int id, double newSourceConfidence) {
    final resp = _execute({
      'action': 'corroborate',
      'id': id,
      'new_source_confidence': newSourceConfidence,
    });
    return ProvenanceEnvelope.fromMsgpack(resp);
  }

  ProvenanceEnvelope verify(int id, String publicKeyHex) {
    final resp = _execute({
      'action': 'verify',
      'id': id,
      'public_key': publicKeyHex,
    });
    return ProvenanceEnvelope.fromMsgpack(resp);
  }

  void updateConfidence(int id, double confidence) {
    _execute({
      'action': 'update_confidence',
      'id': id,
      'confidence': confidence,
    });
  }

  void delete(int id) {
    _execute({'action': 'delete', 'id': id});
  }

  List<ProvenanceEnvelope> query({
    String? collection,
    String? sourceType,
    String? verificationStatus,
    double? minConfidence,
  }) {
    final resp = _execute({
      'action': 'query',
      if (collection != null) 'collection': collection,
      if (sourceType != null) 'source_type': sourceType,
      if (verificationStatus != null)
        'verification_status': verificationStatus,
      if (minConfidence != null) 'min_confidence': minConfidence,
    });
    if (resp is! List) return [];
    return resp.map((e) => ProvenanceEnvelope.fromMsgpack(e)).toList();
  }

  int count() {
    final resp = _execute({'action': 'count'});
    return (resp is int) ? resp : 0;
  }

  String computeHash(dynamic data) {
    final resp = _execute({'action': 'compute_hash', 'data': data});
    return resp as String;
  }

  void close() {
    _bindings.provenanceClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.provenanceExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
