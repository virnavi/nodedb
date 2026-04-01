import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/key_entry.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB KeyResolver engine.
class KeyResolverEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  KeyResolverEngine._(this._handle, this._bindings);

  /// Attach to an existing key resolver engine handle (for multi-isolate use).
  factory KeyResolverEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return KeyResolverEngine._(handle, bindings);
  }

  static KeyResolverEngine open(NodeDbBindings bindings, String path) {
    final handle =
        openRaw(bindings, bindings.keyresolverOpen, buildConfig(path));
    return KeyResolverEngine._(handle, bindings);
  }

  int get handle => _handle;

  KeyEntry supplyKey({
    required String pkiId,
    required String userId,
    required String publicKeyHex,
    String trustLevel = 'explicit',
    String? expiresAtUtc,
  }) {
    final resp = _execute({
      'action': 'supply_key',
      'pki_id': pkiId,
      'user_id': userId,
      'public_key_hex': publicKeyHex,
      'trust_level': trustLevel,
      if (expiresAtUtc != null) 'expires_at_utc': expiresAtUtc,
    });
    return KeyEntry.fromMsgpack(resp);
  }

  KeyEntry? getKey(String pkiId, String userId) {
    final resp = _execute({
      'action': 'get_key',
      'pki_id': pkiId,
      'user_id': userId,
    });
    if (resp == null) return null;
    return KeyEntry.fromMsgpack(resp);
  }

  List<KeyEntry> allKeys() {
    final resp = _execute({'action': 'all_keys'});
    if (resp is! List) return [];
    return resp.map((k) => KeyEntry.fromMsgpack(k)).toList();
  }

  int keyCount() {
    final resp = _execute({'action': 'key_count'});
    return (resp is int) ? resp : 0;
  }

  void revokeKey(String pkiId, String userId) {
    _execute({
      'action': 'revoke_key',
      'pki_id': pkiId,
      'user_id': userId,
    });
  }

  void deleteKey(int id) {
    _execute({'action': 'delete_key', 'id': id});
  }

  void setTrustAll({bool enabled = false}) {
    _execute({'action': 'set_trust_all', 'enabled': enabled});
  }

  void setTrustAllForPeer(String peerId, {bool enabled = false}) {
    _execute({
      'action': 'set_trust_all_for_peer',
      'peer_id': peerId,
      'enabled': enabled,
    });
  }

  bool isTrustAllActive() {
    final resp = _execute({'action': 'is_trust_all_active'});
    return resp == true;
  }

  dynamic verifyWithCache({
    required int provenanceHandle,
    required int envelopeId,
  }) {
    return _execute({
      'action': 'verify_with_cache',
      'provenance_handle': provenanceHandle,
      'envelope_id': envelopeId,
    });
  }

  void close() {
    _bindings.keyresolverClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.keyresolverExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
