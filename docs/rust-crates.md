# Rust Crates Reference

[← Back to Index](README.md)

NodeDB's Rust workspace contains 14 crates under `rust/crates/`. All crates share edition 2021, MIT license, and v0.1.0.

## Dependency Graph

```
                    ┌──────────────┐
                    │  nodedb-ffi  │  (Level 3: Integration)
                    └──────┬───────┘
           ┌───────────────┼───────────────┐
           │               │               │
    ┌──────┴──────┐ ┌──────┴──────┐ ┌──────┴──────────┐
    │ ai-provenance│ │  ai-query   │ │   transport     │  (Level 2)
    └──────┬──────┘ └──────┬──────┘ └──────┬──────────┘
           │          ┌────┴────┐     ┌────┴────┐
    ┌──────┴──────┐   │        │     │         │
    │ provenance  │   │ nosql  │  federation  crypto
    └──────┬──────┘   └───┬────┘     │
           │              │          │
    ┌──────┴──────────────┴──────────┴─────────────┐
    │  graph │ vector │ dac │ keyresolver           │  (Level 1)
    └────────┴────────┴─────┴──────────┬───────────┘
                                       │
                              ┌────────┴────────┐
                              │  nodedb-storage  │  (Level 0)
                              │  nodedb-crypto   │
                              └─────────────────┘
```

## Crate Reference

### nodedb-storage

**Purpose**: Low-level persistent storage over sled with optional encryption.

| Export | Description |
|--------|-------------|
| `StorageEngine` | Open/manage sled databases |
| `StorageTree` | Encrypted tree wrapper |
| `DbHeader` | Database metadata (sealed DEK, owner fingerprint) |
| `IdGenerator` | Namespace-scoped ID generation |
| `TransactionContext` | Transaction support |
| `OwnerKeyStatus` | `Verified`, `Mismatch`, `Unbound` |
| `MigrationRunner` | Schema migration execution |
| `MigrationOp` | `RenameTree`, `DropTree`, etc. |
| `validate_database_name()` | Name validation (lowercase alphanum, hyphens, underscores, max 64) |

**Dependencies**: sled, serde, rmp-serde, aes-gcm, rand

---

### nodedb-crypto

**Purpose**: Cryptographic primitives — identity, signing, encryption.

| Export | Description |
|--------|-------------|
| `NodeIdentity` | Ed25519 keypair (generate, from_bytes, sign) |
| `PublicIdentity` | Public key (peer_id hex, raw bytes) |
| `seal_envelope()` / `open_envelope()` | Public-key encryption |
| `seal_dek()` / `unseal_dek()` | DEK wrapping |
| `fingerprint()` | Key fingerprinting |
| `hkdf_derive_key()` | HKDF key derivation |

**Dependencies**: ed25519-dalek, x25519-dalek, aes-gcm, hkdf, sha2, zeroize

---

### nodedb-nosql

**Purpose**: Document database with schemas, triggers, preferences, and access history.

| Export | Description |
|--------|-------------|
| `Database` | Main facade (open, CRUD, triggers, preferences) |
| `Document` | Record with id, collection, data, timestamps |
| `Collection` | Per-collection document manager |
| `Query` / `Filter` / `FilterCondition` | Query DSL |
| `QualifiedName` | 1/2/3-part collection names |
| `TriggerRegistry` | Trigger management |
| `PreferencesStore` | Per-key encrypted preferences |
| `AccessHistoryStore` | Access event tracking |
| `TrimPolicy` / `TrimReport` | Record pruning |
| `ConflictResolution` | LastWriteWins, LocalWins, RemoteWins, etc. |

**Dependencies**: nodedb-storage, nodedb-crypto

---

### nodedb-graph

**Purpose**: Property graph with nodes, weighted edges, and algorithms.

| Export | Description |
|--------|-------------|
| `GraphEngine` | Node/edge CRUD + algorithms |
| `GraphNode` | Vertex (id, label, data) |
| `GraphEdge` | Edge (id, label, source, target, weight, data) |
| `DeleteBehaviour` | Detach, Restrict, Cascade, Nullify |
| `TraversalResult` | BFS/DFS/shortest path result |
| Algorithms | `bfs`, `dfs`, `shortest_path`, `pagerank`, `connected_components`, `has_cycle`, `find_cycles` |

**Dependencies**: nodedb-storage

---

### nodedb-vector

**Purpose**: HNSW-based approximate nearest-neighbor search.

| Export | Description |
|--------|-------------|
| `VectorEngine` | Insert, search, delete, metadata |
| `VectorRecord` | Vector with metadata |
| `SearchResult` | ID, distance, metadata |
| `DistanceMetric` | Cosine, Euclidean, DotProduct |
| `CollectionConfig` | HNSW hyperparameters |

**Dependencies**: nodedb-storage, hnsw_rs, anndists

---

### nodedb-federation

**Purpose**: Peer and group management for federated networks.

| Export | Description |
|--------|-------------|
| `FederationEngine` | Peer/group lifecycle |
| `NodePeer` | Peer record (name, endpoint, public_key, status) |
| `NodeGroup` | Group record (name, member_ids) |
| `PeerStatus` | active, inactive, banned, unknown |
| `PeerManager` | Name-indexed peer CRUD |
| `GroupManager` | Group CRUD and membership |

**Dependencies**: nodedb-storage

---

### nodedb-dac

**Purpose**: Discretionary Access Control with role/peer-based rules.

| Export | Description |
|--------|-------------|
| `DacEngine` | Rule evaluation and document filtering |
| `NodeAccessRule` | Rule definition (collection, field, record_id, subject, permission) |
| `AccessSubjectType` | peer, group |
| `AccessPermission` | allow, deny, redact |
| `DacSubject` | Subject descriptor |

**Dependencies**: nodedb-storage

---

### nodedb-provenance

**Purpose**: Data lineage tracking with confidence scoring and verification.

| Export | Description |
|--------|-------------|
| `ProvenanceEngine` | Envelope CRUD, confidence, verification |
| `ProvenanceEnvelope` | 24-field metadata record |
| `ProvenanceSourceType` | peer, import, model, user, sensor, ai_query, unknown |
| `ProvenanceVerificationStatus` | unverified, verified, failed, key_requested, trust_all |
| Confidence functions | `initial_confidence`, `corroborate`, `conflict`, `age_decay` |
| `compute_content_hash()` | SHA-256 of canonical msgpack |
| `verify_signature()` | Ed25519 verification |

**Dependencies**: nodedb-storage, nodedb-crypto

---

### nodedb-keyresolver

**Purpose**: Public key registry for distributed signature verification.

| Export | Description |
|--------|-------------|
| `KeyResolverEngine` | Key CRUD and trust management |
| `NodePublicKeyEntry` | Key record (pki_id, user_id, public_key_hex, trust_level) |
| `KeyTrustLevel` | explicit, trust_all, revoked |
| `KeyResolutionResult` | Lookup result |

**Dependencies**: nodedb-storage

---

### nodedb-ai-provenance

**Purpose**: AI-driven confidence assessment and conflict resolution.

| Export | Description |
|--------|-------------|
| `AiProvenanceEngine` | Wraps ProvenanceEngine for AI augmentation |
| `AiProvenanceConfig` | Blend weight, enabled collections |
| `AiProvenanceAssessment` | Suggested confidence + reasoning |
| `AiConflictResolution` | Delta adjustments for two envelopes |
| `AiAnomalyFlag` | Anomaly with penalty and severity |
| `blend_confidence()` | `det * (1-w) + ai * w` |

**Dependencies**: nodedb-provenance, nodedb-storage

---

### nodedb-ai-query

**Purpose**: AI model integration for result processing and schema validation.

| Export | Description |
|--------|-------------|
| `AiQueryEngine` | Wraps Database + ProvenanceEngine |
| `AiQueryConfig` | Min confidence, max results, enabled collections |
| `AiQueryResult` | Result with confidence and metadata |
| `AiQuerySchema` | JSON schema for validation |
| `validate()` | Schema validation function |

**Dependencies**: nodedb-nosql, nodedb-provenance, nodedb-storage

---

### nodedb-transport

**Purpose**: WebSocket P2P networking, federation, mesh, gossip, and pairing.

| Export | Description |
|--------|-------------|
| `TransportEngine` | Async server + gossip + discovery + pairing |
| `TransportConfig` | Full network configuration |
| `PairingStore` / `PairingRecord` | Persistent device pairing |
| `ConnectionPool` | Active peer connections |
| `FederatedRouter` | Multi-hop query forwarding |
| `MeshConfig` / `MeshRouter` | Database-aware mesh routing |
| `GossipManager` | Peer list broadcasting |
| `WireMessage` / `WireMessageType` | Protocol messages |
| `CredentialStore` | Peer acceptance callbacks |
| `AuditLog` | Connection event logging |
| `HandshakeResult` / `AcceptorHandshakeResult` | Handshake outcomes |

**Dependencies**: nodedb-storage, nodedb-crypto, nodedb-federation, tokio, tokio-tungstenite, tokio-rustls, rcgen, mdns-sd, dashmap

---

### nodedb-ffi

**Purpose**: C-compatible FFI bridge exposing all engines via MessagePack.

| Export | Description |
|--------|-------------|
| `nodedb_db_open/close/execute` | NoSQL FFI |
| `nodedb_graph_open/close/execute` | Graph FFI |
| `nodedb_vector_open/close/execute` | Vector FFI |
| `nodedb_transport_open/close/execute` | Transport FFI |
| `nodedb_provenance_open/close/execute` | Provenance FFI |
| ... and 5 more engines | Same pattern |
| `nodedb_write_txn` | Atomic write transaction |
| `nodedb_link_transport` | Link transport to NoSQL |
| `NodeDbError` / error codes | Error reporting |

**Dependencies**: All 12 crates above + tokio, rmp-serde, rmpv

**Library output**: cdylib + staticlib + rlib

## Related Pages

- [Dart Packages](dart-packages.md) — Dart-side wrappers
- [FFI Protocol](ffi.md) — MessagePack protocol details
- [Architecture](architecture.md) — dependency graph and data flow
