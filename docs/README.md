# NodeDB Documentation

NodeDB is a modular, encrypted, federated document database with graph, vector search, provenance tracking, and AI integration — built in Rust with Dart/Flutter bindings.

## Table of Contents

### Getting Started

- [How to Use NodeDB](how_to_use.md) — Comprehensive usage guide with examples
- [Getting Started](getting-started.md) — Installation, setup, and your first database
- [Architecture Overview](architecture.md) — System design, crate/package structure, data flow

### Core Engines

- [NoSQL Engine](nosql-engine.md) — Document storage, collections, schemas, singletons, preferences
- [Graph Engine](graph-engine.md) — Property graph with nodes, edges, and algorithms
- [Vector Engine](vector-engine.md) — HNSW-based approximate nearest-neighbor search
- [Query System](query-system.md) — FilterQuery builder, sorting, pagination, federated queries

### Networking & Federation

- [Transport Layer](transport.md) — WebSocket + TLS networking, mDNS, gossip protocol
- [Federation & Mesh](federation.md) — Peer/group management, mesh networking, cross-device queries
- [Device Pairing](transport.md#device-pairing) — Persistent pairing with user approval

### Security & Provenance

- [Security](security.md) — Encryption, identity, access control (DAC), key management
- [Data Provenance](provenance.md) — Origin tracking, confidence scoring, verification

### AI Integration

- [AI Integration](ai-integration.md) — AI query adapters, AI provenance assessment, blending

### Development

- [Code Generation](code-generation.md) — Annotations, build_runner, generated DAOs and queries
- [FFI Protocol](ffi.md) — MessagePack-based FFI bridge between Rust and Dart
- [Debug Inspector](inspector.md) — Runtime inspection panels, HTTP/WebSocket server, Flutter UI

### Reference

- [Rust Crates](rust-crates.md) — All 14 Rust crates with APIs and dependency graph
- [Dart Packages](dart-packages.md) — All 8 Dart packages with class reference

## Project Structure

```
database/
├── rust/                          # Rust workspace (14 crates)
│   ├── Cargo.toml                 # Workspace manifest
│   └── crates/
│       ├── nodedb-storage/        # Sled-backed storage + encryption
│       ├── nodedb-crypto/         # Ed25519, X25519, AES-256-GCM
│       ├── nodedb-nosql/          # Document DB + triggers + preferences
│       ├── nodedb-graph/          # Property graph + algorithms
│       ├── nodedb-vector/         # HNSW vector search
│       ├── nodedb-federation/     # Peer/group management
│       ├── nodedb-dac/            # Discretionary access control
│       ├── nodedb-transport/      # WebSocket P2P networking
│       ├── nodedb-provenance/     # Data lineage tracking
│       ├── nodedb-keyresolver/    # Public key registry
│       ├── nodedb-ai-provenance/  # AI confidence assessment
│       ├── nodedb-ai-query/       # AI query processing
│       └── nodedb-ffi/            # C-compatible FFI bridge
│
├── packages/                      # Dart/Flutter packages
│   ├── nodedb_ffi/                # Raw dart:ffi bindings
│   ├── nodedb/                    # Core library (facade, engines, models)
│   ├── nodedb_generator/          # Code generation (build_runner)
│   ├── nodedb_flutter_libs/       # Pre-compiled native binaries
│   ├── nodedb_test/               # Test utilities
│   ├── nodedb_inspector/          # Debug data layer (14 panels)
│   ├── nodedb_inspector_flutter/  # Flutter debug UI
│   └── nodedb_example/            # Example Flutter app
│
└── docs/                          # This documentation
```

## Key Features

| Feature | Description |
|---------|-------------|
| **NoSQL Documents** | Schema-optional document storage with collections and transactions |
| **Property Graph** | Nodes, weighted edges, BFS/DFS, PageRank, cycle detection |
| **Vector Search** | HNSW-based ANN with cosine, euclidean, and dot-product metrics |
| **Encryption** | AES-256-GCM at rest, per-preference HKDF keys, Ed25519 identity |
| **Federation** | Peer-to-peer mesh networking with gossip, mDNS, and query forwarding |
| **Provenance** | Per-record origin tracking, confidence scoring, PKI verification |
| **AI Integration** | Pluggable adapters for AI-powered queries and provenance assessment |
| **Code Generation** | Annotations → typed DAOs, filter extensions, serialization |
| **Access Control** | Collection/field/record-level DAC with expiration |
| **Triggers** | Before/after/instead triggers with reentrancy protection |
| **Debug Inspector** | 14-panel runtime inspector with HTTP/WS and Flutter UI |

## Test Coverage

- **702** Rust tests across all crates
- **553** Dart tests (212 generator + 236 nodedb + 77 inspector + 28 inspector_flutter)
- **1,255** total tests
