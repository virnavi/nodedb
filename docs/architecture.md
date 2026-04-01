# Architecture Overview

[← Back to Index](README.md)

NodeDB is a layered system: a Rust core compiled to a shared library, accessed via FFI from Dart/Flutter. Each concern is isolated into its own crate/package, composed at the top level through a facade pattern.

## Layer Diagram

```
┌──────────────────────────────────────────────────────────┐
│  Flutter App  (nodedb_example)                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  NodeDB Facade  (nodedb package)                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │  │
│  │  │ NoSqlEng │ │ GraphEng │ │ TransportEngine   │  │  │
│  │  │ VectorEng│ │ FedEng   │ │ ProvenanceEngine  │  │  │
│  │  │ DacEng   │ │ KeyResEng│ │ AiQuery/AiProv    │  │  │
│  │  └──────────┘ └──────────┘ └───────────────────┘  │  │
│  └──────────────────┬─────────────────────────────────┘  │
│                     │ MessagePack FFI                     │
│  ┌──────────────────┴─────────────────────────────────┐  │
│  │  nodedb_ffi  (dart:ffi bindings)                   │  │
│  └──────────────────┬─────────────────────────────────┘  │
└─────────────────────┼────────────────────────────────────┘
                      │ C ABI (libnodedb_ffi.so/.dylib)
┌─────────────────────┼────────────────────────────────────┐
│  Rust FFI Layer  (nodedb-ffi)                            │
│  ┌──────────────────┴─────────────────────────────────┐  │
│  │  Handle Maps + MessagePack Serialization           │  │
│  └──────────────────┬─────────────────────────────────┘  │
│                     │                                     │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────────┐  │
│  │ nosql   │ │ graph   │ │ vector  │ │ transport    │  │
│  │ fed     │ │ dac     │ │ prov    │ │ keyresolver  │  │
│  │ ai-prov │ │ ai-query│ │ crypto  │ │ storage      │  │
│  └────┬────┘ └────┬────┘ └────┬────┘ └──────┬───────┘  │
│       └───────────┴───────────┴──────────────┘           │
│                        sled (embedded KV)                 │
└──────────────────────────────────────────────────────────┘
```

## Crate Dependency Graph

```
Level 0 — Foundation:
  nodedb-storage ─── sled (embedded key-value store)
  nodedb-crypto  ─── ed25519-dalek, x25519-dalek, aes-gcm

Level 1 — Storage-based engines:
  nodedb-nosql       → nodedb-storage, nodedb-crypto
  nodedb-graph       → nodedb-storage
  nodedb-vector      → nodedb-storage
  nodedb-federation  → nodedb-storage
  nodedb-dac         → nodedb-storage
  nodedb-provenance  → nodedb-storage, nodedb-crypto
  nodedb-keyresolver → nodedb-storage

Level 2 — Composite engines:
  nodedb-transport      → nodedb-storage, nodedb-crypto, nodedb-federation
  nodedb-ai-provenance  → nodedb-provenance, nodedb-storage
  nodedb-ai-query       → nodedb-nosql, nodedb-provenance, nodedb-storage

Level 3 — Integration:
  nodedb-ffi → all crates above
```

## Dart Package Dependency Graph

```
inspector_sdk (pure Dart — abstract interfaces)
    ↓
nodedb_inspector (panels implement InspectorPanel, uses PanelRegistry)
    ↓
nodedb_inspector_flutter (PanelWidgetRegistry, registry-driven UI)
```

## Data Flow

### Write Path

```
Dart writeTxn([WriteOp.put('users', data: {...})])
  → msgpack encode → FFI call → nodedb-ffi
    → Database.trigger_put()
      → TriggerRegistry: fire BEFORE triggers (can modify/abort)
      → Collection.put() → StorageTree.insert() → sled
      → TriggerRegistry: fire AFTER triggers (notifications)
    → if transport linked: emit_trigger_notification() → peers
```

### Read Path

```
Dart findAll('public.users', filter: {...}, sort: [...])
  → msgpack encode → FFI call → nodedb-ffi
    → Database.query() → parse_filter() → Collection.scan()
      → StorageTree.iter() → sled
      → filter match → sort → offset/limit
    → msgpack encode results → Dart
```

### Federated Query Path

```
Dart findAllFederated('public.products', filter: {...})
  → NoSqlEngine.findAll() (local results)
  → TransportEngine.federatedQuery()
    → FederatedRouter.query_peers()
      → for each connected peer (respecting TTL, visited set):
          → WireMessage::QueryRequest → WebSocket → peer
          → peer processes locally → WireMessage::QueryResponse
      → merge results (deduplicate by content hash)
  → combine local + remote → return FederatedResult<Document> list
```

### Provenance Path

```
Dart provenance.attach(collection, recordId, sourceId, ...)
  → compute_content_hash(record data)
  → build_signature_payload(hash | timestamp | pkiId | userId)
  → identity.sign(payload)
  → ProvenanceEnvelope { confidence, signature, hash, ... }
  → sled persist

On verification:
  → keyresolver.resolvePublicKey(pkiId)
  → ed25519_verify(signature, payload, publicKey)
  → update verificationStatus
```

## Design Principles

1. **Modular composition** — Each engine is independent. Enable only what you need.
2. **Encryption by default** — AES-256-GCM at rest, TLS in transit, per-key HKDF for preferences.
3. **Offline-first** — Everything works locally. Mesh networking is additive.
4. **Provenance-first** — Every record can carry origin, confidence, and verification metadata.
5. **Zero-copy FFI** — MessagePack serialization across the Rust↔Dart boundary.
6. **Code generation** — Annotations drive typed DAOs, filters, and serialization.

## Related Pages

- [Rust Crates Reference](rust-crates.md)
- [Dart Packages Reference](dart-packages.md)
- [FFI Protocol](ffi.md)
