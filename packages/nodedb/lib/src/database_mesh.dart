import 'dart:io';

import 'package:nodedb_ffi/nodedb_ffi.dart';

import 'engine/federation_engine.dart';
import 'engine/nosql_engine.dart';
import 'model/mesh_config.dart';
import 'model/node_peer.dart';
import 'model/node_group.dart';
import 'model/transport_config.dart';
import 'p2p/p2p_message_store.dart';

/// Coordinates multiple [NodeDB] instances in a single mesh network.
///
/// Owns the shared [TransportConfig], [FederationEngine], and automatic
/// "all" group for peer management. Databases register with the mesh
/// and communicate with remote devices through it.
///
/// ```dart
/// final mesh = DatabaseMesh.open(
///   directory: '$baseDir/mesh',
///   config: const MeshConfig(meshName: 'my-app'),
///   transportConfig: const TransportConfig(
///     listenAddr: '0.0.0.0:9400',
///     mdnsEnabled: true,
///   ),
/// );
///
/// final db = NodeDB.open(
///   directory: '$baseDir/users',
///   mesh: mesh,
///   databaseName: 'users',
/// );
/// ```
class DatabaseMesh {
  /// Mesh configuration (name, secret, owner key).
  final MeshConfig config;

  /// Transport configuration shared by all databases in this mesh.
  final TransportConfig transportConfig;

  /// Shared federation engine for peer/group management.
  final FederationEngine federation;

  /// Internal mesh database for mesh-level data (P2P messages, etc.).
  late final NoSqlEngine _meshNosql;

  /// Built-in P2P message store for async request/response messaging.
  late final P2pMessageStore messageStore;

  final NodeDbBindings _bindings;
  final String _directory;

  int _nextPortOffset = 0;
  late final int _allGroupId;

  DatabaseMesh._({
    required this.config,
    required this.transportConfig,
    required this.federation,
    required NodeDbBindings bindings,
    required String directory,
  })  : _bindings = bindings,
        _directory = directory;

  /// The FFI bindings used by this mesh.
  NodeDbBindings get bindings => _bindings;

  /// The directory where mesh state (federation) is stored.
  String get directory => _directory;

  /// Open a mesh coordinator.
  ///
  /// Creates or reuses a [FederationEngine] at `directory/__federation__/`
  /// and ensures an "all" group exists for automatic peer membership.
  /// The mesh name from [config].
  String get meshName => config.meshName;

  /// The mesh secret from [config].
  String? get meshSecret => config.meshSecret;

  /// The owner private key hex from [config].
  String? get ownerPrivateKeyHex => config.ownerPrivateKeyHex;

  static DatabaseMesh open({
    required String directory,
    required MeshConfig config,
    required TransportConfig transportConfig,
    NodeDbBindings? bindings,
  }) {
    final b = bindings ?? NodeDbBindings(loadNodeDbLibrary());

    final fedDir = Directory('$directory/__federation__');
    if (!fedDir.existsSync()) fedDir.createSync(recursive: true);
    final federation = FederationEngine.open(b, fedDir.path);

    final mesh = DatabaseMesh._(
      config: config,
      transportConfig: transportConfig,
      federation: federation,
      bindings: b,
      directory: directory,
    );

    // Open internal mesh database for mesh-level data storage
    final meshDataDir = Directory('$directory/__mesh_data__');
    if (!meshDataDir.existsSync()) meshDataDir.createSync(recursive: true);
    mesh._meshNosql = NoSqlEngine.open(b, meshDataDir.path);
    mesh.messageStore = P2pMessageStore(mesh._meshNosql);

    mesh._ensureAllGroup();
    return mesh;
  }

  void _ensureAllGroup() {
    final existing = federation.getGroupByName('all');
    if (existing != null) {
      _allGroupId = existing.id;
    } else {
      final group = federation.addGroup('all');
      _allGroupId = group.id;
    }
  }

  /// The ID of the automatic "all" group.
  int get allGroupId => _allGroupId;

  /// Allocate a listen address for the next database.
  ///
  /// Parses host:port from [transportConfig.listenAddr] and increments the
  /// port for each call. First database gets the base port, second gets
  /// base+1, etc.
  String allocateListenAddr() {
    final parts = transportConfig.listenAddr.split(':');
    final host = parts.length > 1 ? parts[0] : '0.0.0.0';
    final basePort =
        int.tryParse(parts.length > 1 ? parts[1] : parts[0]) ?? 9400;
    final addr = '$host:${basePort + _nextPortOffset}';
    _nextPortOffset++;
    return addr;
  }

  // ── Peer management with auto-group ──────────────────────────

  /// Add a peer and automatically add it to the "all" group.
  NodePeer addPeer(String name, String endpoint,
      {String status = 'active'}) {
    final peer = federation.addPeer(name, endpoint, status: status);
    federation.addMember(_allGroupId, peer.id);
    return peer;
  }

  /// Remove a peer from all groups and delete it.
  void removePeer(int peerId) {
    final groupIds = federation.groupsForPeer(peerId);
    for (final gid in groupIds) {
      federation.removeMember(gid, peerId);
    }
    federation.deletePeer(peerId);
  }

  // ── Federation delegates ─────────────────────────────────────

  /// Get a peer by ID.
  NodePeer? getPeer(int id) => federation.getPeer(id);

  /// Get a peer by name.
  NodePeer? getPeerByName(String name) => federation.getPeerByName(name);

  /// Update a peer's status and/or endpoint.
  void updatePeer(int id, {String? status, String? endpoint}) =>
      federation.updatePeer(id, status: status, endpoint: endpoint);

  /// List all peers.
  List<NodePeer> allPeers() => federation.allPeers();

  /// Count all peers.
  int peerCount() => federation.peerCount();

  /// Add a group.
  NodeGroup addGroup(String name, {Map<String, dynamic>? metadata}) =>
      federation.addGroup(name, metadata: metadata);

  /// Get a group by ID.
  NodeGroup? getGroup(int id) => federation.getGroup(id);

  /// Get a group by name.
  NodeGroup? getGroupByName(String name) => federation.getGroupByName(name);

  /// Update a group.
  void updateGroup(int id, {String? name, Map<String, dynamic>? metadata}) =>
      federation.updateGroup(id, name: name, metadata: metadata);

  /// Delete a group.
  void deleteGroup(int id) => federation.deleteGroup(id);

  /// List all groups.
  List<NodeGroup> allGroups() => federation.allGroups();

  /// Count all groups.
  int groupCount() => federation.groupCount();

  /// Add a peer to a group.
  void addMember(int groupId, int peerId) =>
      federation.addMember(groupId, peerId);

  /// Remove a peer from a group.
  void removeMember(int groupId, int peerId) =>
      federation.removeMember(groupId, peerId);

  /// Get group IDs that a peer belongs to.
  List<int> groupsForPeer(int peerId) => federation.groupsForPeer(peerId);

  /// The internal mesh NoSQL engine handle (for cross-isolate sharing).
  int get meshNosqlHandle => _meshNosql.handle;

  /// Close the mesh and release resources.
  ///
  /// Does not close individual databases — close those separately.
  void close() {
    _meshNosql.close();
    federation.close();
  }
}
