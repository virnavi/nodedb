import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/node_peer.dart';
import '../model/node_group.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB Federation engine.
class FederationEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  FederationEngine._(this._handle, this._bindings);

  /// Attach to an existing federation engine handle (for multi-isolate use).
  factory FederationEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return FederationEngine._(handle, bindings);
  }

  static FederationEngine open(NodeDbBindings bindings, String path) {
    final handle = openRaw(bindings, bindings.federationOpen, buildConfig(path));
    return FederationEngine._(handle, bindings);
  }

  int get handle => _handle;

  // ── Peer operations ─────────────────────────────────────────────

  NodePeer addPeer(String name, String endpoint, {String status = 'active'}) {
    final resp = _execute({
      'action': 'add_peer',
      'name': name,
      'endpoint': endpoint,
      'status': status,
    });
    return NodePeer.fromMsgpack(resp);
  }

  NodePeer? getPeer(int id) {
    try {
      final resp = _execute({'action': 'get_peer', 'id': id});
      if (resp == null) return null;
      return NodePeer.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  NodePeer? getPeerByName(String name) {
    try {
      final resp = _execute({'action': 'get_peer_by_name', 'name': name});
      if (resp == null) return null;
      return NodePeer.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  void updatePeer(int id, {String? status, String? endpoint}) {
    _execute({
      'action': 'update_peer',
      'id': id,
      if (status != null) 'status': status,
      if (endpoint != null) 'endpoint': endpoint,
    });
  }

  void deletePeer(int id) {
    _execute({'action': 'delete_peer', 'id': id});
  }

  List<NodePeer> allPeers() {
    final resp = _execute({'action': 'all_peers'});
    if (resp is! List) return [];
    return resp.map((p) => NodePeer.fromMsgpack(p)).toList();
  }

  int peerCount() {
    final resp = _execute({'action': 'peer_count'});
    return (resp is int) ? resp : 0;
  }

  // ── Group operations ────────────────────────────────────────────

  NodeGroup addGroup(String name, {Map<String, dynamic>? metadata}) {
    final resp = _execute({
      'action': 'add_group',
      'name': name,
      if (metadata != null) 'metadata': metadata,
    });
    return NodeGroup.fromMsgpack(resp);
  }

  NodeGroup? getGroup(int id) {
    try {
      final resp = _execute({'action': 'get_group', 'id': id});
      if (resp == null) return null;
      return NodeGroup.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  NodeGroup? getGroupByName(String name) {
    try {
      final resp = _execute({'action': 'get_group_by_name', 'name': name});
      if (resp == null) return null;
      return NodeGroup.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  void updateGroup(int id, {String? name, Map<String, dynamic>? metadata}) {
    _execute({
      'action': 'update_group',
      'id': id,
      if (name != null) 'name': name,
      if (metadata != null) 'metadata': metadata,
    });
  }

  void deleteGroup(int id) {
    _execute({'action': 'delete_group', 'id': id});
  }

  List<NodeGroup> allGroups() {
    final resp = _execute({'action': 'all_groups'});
    if (resp is! List) return [];
    return resp.map((g) => NodeGroup.fromMsgpack(g)).toList();
  }

  int groupCount() {
    final resp = _execute({'action': 'group_count'});
    return (resp is int) ? resp : 0;
  }

  // ── Membership operations ───────────────────────────────────────

  void addMember(int groupId, int peerId) {
    _execute({'action': 'add_member', 'group_id': groupId, 'peer_id': peerId});
  }

  void removeMember(int groupId, int peerId) {
    _execute({
      'action': 'remove_member',
      'group_id': groupId,
      'peer_id': peerId,
    });
  }

  /// Returns group IDs that the peer belongs to.
  List<int> groupsForPeer(int peerId) {
    final resp = _execute({'action': 'groups_for_peer', 'peer_id': peerId});
    if (resp is! List) return [];
    return resp.map((id) => (id as num).toInt()).toList();
  }

  void close() {
    _bindings.federationClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.federationExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
