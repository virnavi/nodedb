import 'dart:typed_data';

import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB Transport engine.
class TransportEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  TransportEngine._(this._handle, this._bindings);

  /// Attach to an existing transport engine handle (for multi-isolate use).
  factory TransportEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return TransportEngine._(handle, bindings);
  }

  static TransportEngine open(
    NodeDbBindings bindings,
    Map<String, dynamic> config,
  ) {
    final configBytes = msgpackEncode(config);
    final handle = openRaw(bindings, bindings.transportOpen, configBytes);
    return TransportEngine._(handle, bindings);
  }

  int get handle => _handle;

  Map<String, dynamic> identity() {
    final resp = _execute({'action': 'identity'});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  List<dynamic> connectedPeers() {
    final resp = _execute({'action': 'connected_peers'});
    if (resp is List) return resp;
    if (resp is Map) {
      final peerIds = resp['peer_ids'];
      if (peerIds is List) return peerIds;
    }
    return [];
  }

  List<dynamic> knownPeers() {
    final resp = _execute({'action': 'known_peers'});
    if (resp is List) return resp;
    if (resp is Map) {
      final peers = resp['peers'];
      if (peers is List) return peers;
    }
    return [];
  }

  /// List all discovered peers (mDNS + seed + gossip).
  List<Map<String, dynamic>> discoveredPeers() {
    final resp = _execute({'action': 'discovered_peers'});
    if (resp is! Map) return [];
    final peers = resp['peers'];
    if (peers is! List) return [];
    return peers
        .map((e) => e is Map ? Map<String, dynamic>.from(e) : <String, dynamic>{})
        .toList();
  }

  Map<String, dynamic> meshStatus() {
    final resp = _execute({'action': 'mesh_status'});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  List<dynamic> meshMembers() {
    final resp = _execute({'action': 'mesh_members'});
    if (resp is List) return resp;
    return [];
  }

  dynamic meshQuery({
    required String database,
    required String queryType,
    required Map<String, dynamic> queryData,
    int timeoutSecs = 10,
  }) {
    final raw = _execute({
      'action': 'mesh_query',
      'database': database,
      'query_type': queryType,
      'query_data': queryData,
      'timeout_secs': timeoutSecs,
    });
    // Decode binary result blobs from each peer into Dart objects
    if (raw is Map) {
      final results = raw['results'];
      if (results is List) {
        final decoded = <dynamic>[];
        for (final r in results) {
          if (r is Uint8List && r.isNotEmpty) {
            try {
              final val = msgpackDecode(r);
              if (val is List) {
                decoded.addAll(val);
              } else if (val != null) {
                decoded.add(val);
              }
            } catch (_) {}
          } else if (r is List || r is Map) {
            decoded.add(r);
          }
        }
        return <String, dynamic>{...Map<String, dynamic>.from(raw), 'results': decoded};
      }
    }
    return raw;
  }

  dynamic federatedQuery({
    required String queryType,
    required Map<String, dynamic> queryData,
    int timeoutSecs = 10,
    int ttl = 3,
  }) {
    return _execute({
      'action': 'federated_query',
      'query_type': queryType,
      'query_data': queryData,
      'timeout_secs': timeoutSecs,
      'ttl': ttl,
    });
  }

  Map<String, dynamic> connect(String endpoint) {
    final resp = _execute({'action': 'connect', 'endpoint': endpoint});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  List<Map<String, dynamic>> auditLog({int? limit}) {
    final resp = _execute({
      'action': 'audit_log',
      if (limit != null) 'limit': limit,
    });
    if (resp is! List) return [];
    return resp
        .map((e) => e is Map ? Map<String, dynamic>.from(e) : <String, dynamic>{})
        .toList();
  }

  void setCredential(String peerId, String token) {
    _execute({'action': 'set_credential', 'peer_id': peerId, 'token': token});
  }

  /// Update the mesh secret at runtime (e.g., after connecting via QR invite).
  void setMeshSecret(String secret) {
    _execute({'action': 'set_mesh_secret', 'secret': secret});
  }

  /// Register a device directly in the pairing store (pre-authorize).
  /// Once registered, the device can connect and pass the handshake,
  /// gaining access to gossip and federation.
  Map<String, dynamic> registerDevice({
    required String peerId,
    String publicKeyHex = '',
    String userId = '',
    String deviceName = '',
  }) {
    final resp = _execute({
      'action': 'register_device',
      'peer_id': peerId,
      'public_key_hex': publicKeyHex,
      'user_id': userId,
      'device_name': deviceName,
    });
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  /// List all persistently paired devices.
  List<Map<String, dynamic>> pairedDevices() {
    final resp = _execute({'action': 'paired_devices'});
    if (resp is! Map) return [];
    final devices = resp['devices'];
    if (devices is! List) return [];
    return devices
        .map((e) => e is Map ? Map<String, dynamic>.from(e) : <String, dynamic>{})
        .toList();
  }

  /// List pending pairing requests awaiting user approval.
  List<Map<String, dynamic>> pendingPairings() {
    final resp = _execute({'action': 'pending_pairings'});
    if (resp is! Map) return [];
    final pending = resp['pending'];
    if (pending is! List) return [];
    return pending
        .map((e) => e is Map ? Map<String, dynamic>.from(e) : <String, dynamic>{})
        .toList();
  }

  /// Approve a pending pairing request. Returns the pairing info.
  Map<String, dynamic> approvePairing(String peerId) {
    final resp = _execute({'action': 'approve_pairing', 'peer_id': peerId});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  /// Reject a pending pairing request.
  bool rejectPairing(String peerId) {
    final resp = _execute({'action': 'reject_pairing', 'peer_id': peerId});
    if (resp is Map) return resp['removed'] == true;
    return false;
  }

  /// Remove (unpair) a previously paired device.
  bool removePairedDevice(String peerId) {
    final resp = _execute({'action': 'remove_paired_device', 'peer_id': peerId});
    if (resp is Map) return resp['removed'] == true;
    return false;
  }

  void close() {
    _bindings.transportClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.transportExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
