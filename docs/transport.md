# Transport Layer

[← Back to Index](README.md)

The transport layer provides peer-to-peer networking over WebSocket + TLS, with mDNS auto-discovery, gossip-based peer sharing, and persistent device pairing.

## Configuration

Transport is configured at the mesh level via `DatabaseMesh`, not on individual databases. All databases in a mesh share the same transport configuration and port auto-allocation.

```dart
// 1. Create a mesh with transport config
final mesh = DatabaseMesh.open(
  directory: '$baseDir/mesh',
  config: const MeshConfig(meshName: 'my-app'),
  transportConfig: const TransportConfig(
    listenAddr: '0.0.0.0:9400',       // Base WebSocket listen address
    mdnsEnabled: true,                  // mDNS auto-discovery
    seedPeers: ['wss://192.168.1.5:9400'], // Bootstrap peers
    queryPolicy: QueryPolicy.queryPeersOnMiss,
    gossipIntervalSeconds: 30,          // Peer list broadcast interval
    gossipFanOut: 3,                    // Peers per gossip round
    gossipTtl: 5,                       // Max gossip hops
    identityKeyHex: '...',              // Stable Ed25519 identity (64 hex chars)
    trustedPeerKeys: ['...'],           // Whitelist peer public keys
    requirePairing: true,               // Require user approval for new peers
    userId: 'uuid-v7-here',            // User ID for pairing
    deviceName: 'My Phone',            // Human-readable device name
  ),
);

// 2. Open databases — transport is auto-configured via mesh
final db = NodeDB.open(
  directory: '$baseDir/users',
  databaseName: 'users',
  mesh: mesh,                           // Gets port 9400
);

final db2 = NodeDB.open(
  directory: '$baseDir/products',
  databaseName: 'products',
  mesh: mesh,                           // Gets port 9401 (auto-incremented)
);
```

Port auto-allocation: the mesh parses the base port from `TransportConfig.listenAddr` and increments it for each registered database (first DB gets base port, second gets base+1, etc.).

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `listenAddr` | `0.0.0.0:9400` | Address:port for WebSocket server |
| `mdnsEnabled` | `true` | Auto-discover peers on local network |
| `seedPeers` | `[]` | Bootstrap WSS endpoints |
| `queryPolicy` | `queryPeersOnMiss` | When to query remote peers |
| `gossipIntervalSeconds` | `30` | Broadcast frequency |
| `gossipFanOut` | `3` | Peers per gossip round |
| `gossipTtl` | `5` | Max hops for gossip messages |
| `identityKeyHex` | `null` | Stable Ed25519 key (generated if null) |
| `trustedPeerKeys` | `[]` | Whitelist (empty = accept all) |
| `requirePairing` | `false` | Require user-approved pairing |
| `userId` | `null` | UUID for pairing record |
| `deviceName` | `null` | Human-readable name |

> **Note**: `TransportConfig` is passed to `DatabaseMesh.open()`, not to `NodeDB.open()` directly. The mesh owns the transport configuration and auto-allocates listen ports for each database.

### Query Policies

| Policy | Behaviour |
|--------|-----------|
| `localOnly` | Never query peers |
| `queryPeersOnMiss` | Query peers when local results are empty |
| `queryPeersAlways` | Always query peers in addition to local |
| `queryPeersExplicitly` | Only query peers via `findAllFederated()` |

## Connection Management

```dart
final transport = db.transport!;

// Manual connect to a peer
transport.connect('wss://192.168.1.5:9400');

// View connections
final identity = transport.identity();
// {peer_id: '...', public_key_bytes: [...]}

final connected = transport.connectedPeers();
// [{count: 2, peer_ids: ['...', '...']}]

final known = transport.knownPeers();
// [{peer_id, endpoint, status, ttl}, ...]
```

## Handshake Protocol

Every connection goes through:

1. **TCP connect** → TLS handshake (self-signed certs via rcgen)
2. **WebSocket upgrade** → HTTP Upgrade to WSS
3. **Hello/HelloAck exchange**:
   - Initiator sends `Hello` with: public key, endpoint, nonce, user_id, device_name
   - Acceptor verifies nonce signature, checks credentials/pairing
   - Acceptor sends `HelloAck` with: accepted flag, public key, endpoint, pairing_required flag
4. **Key exchange** → X25519 shared secret derived for encrypted messages

## Device Pairing

When `requirePairing: true`, unknown peers must be approved by the user before connecting.

### Flow

```
Device A (initiator)              Device B (acceptor, pairing enabled)
     │                                   │
     │── Hello (pubkey, userId, name) ──>│
     │                                   │ Check PairingStore:
     │                                   │   Paired? → verify_reconnect() → accept
     │                                   │   Unknown? → add_pending() → reject
     │<── HelloAck (pairing_required) ──│
     │                                   │
     │   (User on Device B approves      │
     │    via approvePairing())           │
     │                                   │ PairingStore: pending → paired (sled)
     │                                   │
     │── Hello (retry) ────────────────>│
     │                                   │ Paired! → verify_reconnect() → accept
     │<── HelloAck (accepted) ─────────│
     │                                   │
     │   Connection established          │
```

### Pairing API

```dart
// List pending pairing requests (devices waiting for approval)
final pending = transport.pendingPairings();
// [{peer_id, user_id, device_name, endpoint, received_at}, ...]

// Approve a pairing request
final result = transport.approvePairing(peerId);
// {ok: true, peer_id: '...', user_id: '...'}

// Reject a pairing request
transport.rejectPairing(peerId);

// List paired devices
final devices = transport.pairedDevices();
// [{peer_id, user_id, device_name, paired_at, last_verified_at}, ...]

// Unpair a device
transport.removePairedDevice(peerId);
```

### Pairing Record

Stored persistently in sled (`__pairing__` tree):

| Field | Type | Description |
|-------|------|-------------|
| `peer_id` | String | Ed25519 public key hex (64 chars) |
| `public_key_bytes` | Vec<u8> | Raw 32-byte public key |
| `user_id` | String | UUID of the paired user |
| `device_name` | String | Human-readable device name |
| `paired_at` | DateTime | When the pairing was approved |
| `last_verified_at` | DateTime | Last successful reconnect verification |

### Reconnect Verification

When a previously paired device reconnects:
1. Look up `PairingRecord` by peer_id
2. Verify `public_key_bytes` match
3. Verify `user_id` matches
4. Update `last_verified_at` timestamp
5. Accept the connection

If either key or user_id mismatch → `PairingVerificationFailed` error.

## Gossip Protocol

Peers share their known peer lists via gossip:

- **Interval**: Configurable (default 30s)
- **Fan-out**: Number of peers to gossip to each round (default 3)
- **TTL**: Max hops a gossip message travels (default 5)
- **Mesh fields**: Each gossip entry includes `database_name`, `mesh_name`, `sharing_status`, `schema_fingerprint`
- **HMAC authentication**: Optional mesh secret for authenticated gossip payloads

## mDNS Discovery

When `mdnsEnabled: true`:
- Registers a `_nodedb._tcp.local.` service
- Discovers other NodeDB instances on the local network
- Auto-connects to discovered peers (~30 seconds)

## Wire Protocol

Messages are MessagePack-encoded `WireMessage` envelopes:

| Type | Direction | Purpose |
|------|-----------|---------|
| `Hello` | Initiator → Acceptor | Identity + nonce exchange |
| `HelloAck` | Acceptor → Initiator | Accept/reject + pairing status |
| `GossipPeerList` | Bidirectional | Share known peers |
| `QueryRequest` | Initiator → Peer | Federated query |
| `QueryResponse` | Peer → Initiator | Query results |
| `Ping` / `Pong` | Bidirectional | Keepalive |
| `TriggerNotification` | Write → Peers | Trigger event broadcast |
| `PreferenceSync` | Bidirectional | Preference synchronization |
| `SingletonSync` | Bidirectional | Singleton synchronization |

## Audit Log

When `storagePath` is set, connection events are logged:

```dart
final logs = transport.auditLog(limit: 50);
// [{event, peer_id, endpoint, timestamp}, ...]
```

## Rust Implementation

**Crate**: `nodedb-transport` → depends on `nodedb-storage`, `nodedb-crypto`, `nodedb-federation`

Key types:
- `TransportEngine` — async server + gossip + discovery + pairing
- `TransportConfig` — full configuration
- `PairingStore` — sled-backed persistent pairing
- `ConnectionPool` — active connection management
- `FederatedRouter` — multi-hop query routing
- `GossipManager` — peer list broadcasting
- `MeshRouter` — database-aware query routing
- `HandshakeResult` / `AcceptorHandshakeResult` — handshake outcomes

## Related Pages

- [Federation & Mesh](federation.md) — mesh networking and query routing
- [Security](security.md) — TLS, identity, encryption
- [Getting Started](getting-started.md) — basic transport setup
