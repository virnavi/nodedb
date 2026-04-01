# Federation & Mesh

[← Back to Index](README.md)

Federation enables peer-to-peer data sharing across multiple NodeDB instances. Mesh networking adds database-aware routing for multi-device collaboration.

## Concepts

### Peers

A **peer** is another NodeDB instance on the network:

```dart
class NodePeer {
  final int id;
  final String name;
  final String endpoint;    // e.g., 'wss://192.168.1.5:9400'
  final String? publicKey;  // Ed25519 public key hex
  final String status;      // active, inactive, banned, unknown
  final Map<String, dynamic> metadata;
}
```

### Groups

A **group** organizes peers for collective management:

```dart
class NodeGroup {
  final int id;
  final String name;
  final List<int> memberIds;  // Peer IDs
  final Map<String, dynamic> metadata;
}
```

### Mesh

A **mesh** is a named overlay network where databases with the same `meshName` coordinate. The `DatabaseMesh` class owns the shared transport and federation:

```dart
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(
    meshName: 'family',              // Mesh identity
    meshSecret: 'optional-secret',   // HMAC authentication for gossip
    ownerPrivateKeyHex: '...',       // Shared owner key (optional)
  ),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',
    mdnsEnabled: true,
  ),
);

final db = NodeDB.open(
  directory: '$baseDir/photos',
  databaseName: 'photos',           // This database's name in the mesh
  sharingStatus: 'full',            // Sharing level
  mesh: mesh,
);
```

### Automatic "all" Group

When a `DatabaseMesh` is opened, it creates an "all" group automatically. Every peer added via `mesh.addPeer()` is auto-added to this group, and `mesh.removePeer()` removes the peer from all groups before deletion.

## Federation Engine

The federation engine manages peers and groups (always enabled via `__mgmt__` directory):

```dart
// Add a peer
final peer = db.federation.addPeer(
  name: 'Bob\'s Phone',
  endpoint: 'wss://192.168.1.5:9400',
  publicKey: '...',
);

// List peers
final peers = db.federation.allPeers();

// Create a group
final group = db.federation.addGroup(name: 'Family');

// Add peer to group
db.federation.addPeerToGroup(group.id, peer.id);

// Get groups for a peer
final groups = db.federation.groupsForPeer(peer.id);
// Returns group IDs (integers)
```

## Mesh Networking

### Configuration

```dart
// Create a mesh with shared transport
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(meshName: 'nodedb-example'),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',
    mdnsEnabled: true,
  ),
);

// Open databases via mesh
final db = NodeDB.open(
  directory: '$baseDir/users',
  databaseName: 'users',
  sharingStatus: 'full',
  mesh: mesh,
);
```

### Sharing Status Cascade

The effective sharing status for a collection is determined by cascading:

1. **Collection-level** (from `@Shareable` annotation)
2. **Schema-level** (from schema config)
3. **Database-level** (`NodeDB.open(sharingStatus: ...)`)
4. **Default**: `full`

### Sharing Levels

| Status | Can Read | Can Write | Can Query |
|--------|----------|-----------|-----------|
| `private` | No | No | No |
| `read_only` | Yes | No | Yes |
| `read_write` | Yes | Yes | Yes |
| `full` | Yes | Yes | Yes (+ admin) |

### Mesh Status

```dart
final status = transport.meshStatus();
// {mesh_name, database_name, sharing_status, peer_count, ...}

final members = transport.meshMembers();
// [{peer_id, database_name, sharing_status, schema_fingerprint}, ...]
```

### Schema Fingerprint

Each database computes a SHA-256 fingerprint of its sorted schema meta keys. Peers exchange fingerprints via gossip to detect schema mismatches across the mesh.

## Federated Queries

### Direct Query

```dart
// Query all connected peers
final results = db.findAllFederated(
  'public.products',
  filter: {'Condition': {'Contains': {'field': 'name', 'value': 'Laptop'}}},
);

// Results tagged with source
for (final r in results) {
  print('${r.data.data['name']} from ${r.sourcePeerId}');
  // sourcePeerId is 'local' or a peer UUID
}
```

### Mesh Query (Database-Targeted)

```dart
// Query a specific database in the mesh by name
final results = transport.meshQuery(
  database: 'products',
  queryType: 'nosql',
  queryData: {
    'collection': 'public.products',
    'query': {'filter': filter},
  },
  timeoutSecs: 10,
);
```

### Multi-Hop Routing

Federated queries support multi-hop forwarding:

- **TTL**: Decremented at each hop (default 3)
- **Visited set**: Prevents loops — each peer adds itself to `visited`
- **Timeout**: 5 seconds per hop
- **Fan-out**: Queries forwarded to all connected peers (minus visited)

```
Peer A ──query──> Peer B ──forward──> Peer C
                                       │
                  Peer D <──forward────┘
```

### Query Handler

The `QueryHandler` trait allows custom query processing:

```rust
pub trait QueryHandler: Send + Sync {
    fn handle_query(&self, query_type: &str, data: &[u8]) -> Result<Vec<u8>>;
    fn merge_results(&self, local: Vec<u8>, remote: Vec<Vec<u8>>) -> Vec<u8>;
}
```

The FFI layer implements `FfiQueryHandler` which dispatches to the appropriate engine based on `query_type`.

## Gossip Protocol

Peers share information through periodic gossip broadcasts:

```
Every 30 seconds:
  1. Select up to 3 random connected peers
  2. Send GossipPeerList with:
     - Known peers (id, endpoint, status, TTL)
     - Mesh info (database_name, mesh_name, sharing_status, schema_fingerprint)
  3. If mesh_secret set: wrap in AuthenticatedGossipPayload with HMAC tag
  4. Receiving peer:
     - Merges new peers into known set
     - Updates MeshRouter with mesh member info
     - Decrements TTL and re-gossips if TTL > 0
```

## Multi-Database Mesh

A single device can run multiple databases in the same mesh using `DatabaseMesh`:

```dart
// Create shared mesh coordinator
final mesh = DatabaseMesh.open(
  directory: '$base/mesh',
  config: const MeshConfig(meshName: 'nodedb-example'),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',
    mdnsEnabled: true,
  ),
);

// Databases share federation + auto-allocate transport ports
final usersDb = NodeDB.open(
  directory: '$base/users',
  databaseName: 'users',
  mesh: mesh,                   // Gets port 9400
);

final productsDb = NodeDB.open(
  directory: '$base/products',
  databaseName: 'products',
  mesh: mesh,                   // Gets port 9401
);

// Peer management through the mesh
final peer = mesh.addPeer('Bob', 'wss://192.168.1.5:9400');
// Peer is auto-added to the "all" group

mesh.removePeer(peer.id);
// Peer is removed from all groups, then deleted
```

Each database gets its own transport engine (different ports, required by Rust FFI) but shares the mesh's federation engine and transport configuration. The mesh auto-increments the listen port for each registered database.

### Without Mesh (Local-Only Mode)

```dart
// No mesh = no transport, local-only federation
final db = NodeDB.open(
  directory: '/path/to/data',
  databaseName: 'local-db',
);
```

## Rust Implementation

**Crate**: `nodedb-federation` → depends on `nodedb-storage`

Key types:
- `FederationEngine` — peer/group CRUD
- `NodePeer`, `NodeGroup` — data models
- `PeerManager` — peer lifecycle with name-indexed lookups
- `GroupManager` — group CRUD and membership

**Crate**: `nodedb-transport` (mesh/routing) → depends on `nodedb-federation`

Key types:
- `MeshConfig`, `MeshStatus`, `MeshRouter` — mesh networking
- `MeshSharingStatus` — sharing level enum
- `FederatedRouter` — multi-hop query forwarding
- `GossipManager` — peer list broadcasting
- `AuthenticatedGossipPayload` — HMAC-protected gossip

## Related Pages

- [Transport Layer](transport.md) — WebSocket + TLS networking, pairing
- [Query System](query-system.md) — filter DSL for federated queries
- [Security](security.md) — peer authentication, mesh secrets
