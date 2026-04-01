# NodeDB Rust Core

## Crate Dependency Graph

```
nodedb-ffi
  ├── nodedb-nosql
  │     ├── nodedb-storage (sled, serde, rmp-serde, thiserror)
  │     └── nodedb-crypto (hkdf)
  ├── nodedb-graph
  │     └── nodedb-storage
  ├── nodedb-vector (hnsw_rs, anndists)
  │     └── nodedb-storage
  ├── nodedb-federation
  │     └── nodedb-storage
  ├── nodedb-dac
  │     └── nodedb-storage
  ├── nodedb-transport (tokio, tokio-tungstenite, rustls, mdns-sd)
  │     ├── nodedb-federation
  │     ├── nodedb-crypto (ed25519-dalek, x25519-dalek, aes-gcm)
  │     └── nodedb-storage
  ├── nodedb-provenance (sha2, chrono)
  │     ├── nodedb-storage
  │     └── nodedb-crypto
  ├── nodedb-ai-provenance (chrono)
  │     └── nodedb-provenance
  ├── nodedb-ai-query (chrono, rmpv)
  │     ├── nodedb-nosql
  │     └── nodedb-provenance
  ├── nodedb-keyresolver (chrono)
  │     └── nodedb-storage
  └── nodedb-crypto
```

## Build

```bash
cargo build
```

Produces:
- `target/debug/libnodedb_ffi.dylib` (macOS) / `libnodedb_ffi.so` (Linux)
- `target/debug/libnodedb_ffi.a` (static library)

## C Header Generation

Install cbindgen and generate the header:

```bash
cargo install cbindgen
cbindgen --crate nodedb-ffi --output nodedb.h
```

## MessagePack FFI Protocol

All data crossing the FFI boundary is serialized as MessagePack.

### Open

Config (MessagePack map):
```json
{"path": "/path/to/database"}
```

### Write Transaction

Operations (MessagePack array):
```json
[
  {"collection": "users", "action": "put", "data": {"name": "Alice", "age": 30}},
  {"collection": "users", "action": "delete", "id": 1}
]
```

### Query

Request (MessagePack map):
```json
{"collection": "users", "action": "find_all", "offset": 0, "limit": 10}
{"collection": "users", "action": "get", "id": 1}
{"collection": "analytics.page_views", "action": "find_all"}
```

Collection names support fully qualified names: `"users"` (defaults to `public.users`), `"analytics.page_views"` (schema.collection), or `"warehouse.public.products"` (database.schema.collection).

### Schema Management

Request (MessagePack map, via Query endpoint):
```json
{"action": "create_schema", "name": "analytics", "sharing_status": "read_only"}
{"action": "drop_schema", "name": "analytics"}
{"action": "list_schemas"}
{"action": "schema_info", "name": "analytics"}
{"action": "move_collection", "from": "public.orders", "to_schema": "analytics"}
{"action": "rename_schema", "from": "internal", "to": "private"}
{"action": "schema_fingerprint"}
{"action": "collection_names"}
{"action": "collection_names_in_schema", "schema": "analytics"}
```

Sharing status cascade: collection-level → schema-level → database MeshConfig → default `full`.

### Trigger Management

Request (MessagePack map, via Query endpoint):
```json
{"action": "register_trigger", "collection": "users", "event": "insert", "timing": "before", "name": "validate_user"}
{"action": "register_mesh_trigger", "source_database": "remote_db", "collection": "orders", "event": "insert", "timing": "after", "name": "sync_orders"}
{"action": "unregister_trigger", "trigger_id": 1}
{"action": "list_triggers"}
{"action": "set_trigger_enabled", "trigger_id": 1, "enabled": false}
```

Triggers fire automatically on write operations (put/delete via `write_txn`). Events: `insert`, `update`, `delete`. Timings: `before` (can modify/abort), `after` (post-write side effects), `instead` (replaces write). FFI-registered triggers are passthrough (proceed); Rust API supports full callbacks. Reentrancy guard: max depth 8.

Mesh triggers: `register_mesh_trigger` registers a trigger that fires when a remote peer sends a `TriggerNotification`. Link a transport to a database via `nodedb_link_transport(db_handle, transport_handle)` to enable automatic notification broadcast on writes.

### Graph Execute

Request (MessagePack map):
```json
{"action": "add_node", "label": "person", "data": {"name": "Alice"}}
{"action": "add_edge", "label": "knows", "source": 1, "target": 2, "weight": 1.0}
{"action": "get_node", "id": 1}
{"action": "edges_from", "id": 1}
{"action": "bfs", "id": 1, "max_depth": 3}
{"action": "shortest_path", "from": 1, "to": 3}
{"action": "pagerank", "damping": 0.85, "iterations": 20}
{"action": "delete_node", "id": 1, "behaviour": "detach"}
```

### Vector Execute

Request (MessagePack map):
```json
{"action": "insert", "vector": [1.0, 0.0, 0.0], "metadata": {"label": "x-axis"}}
{"action": "get", "id": 1}
{"action": "delete", "id": 1}
{"action": "update_metadata", "id": 1, "metadata": {"label": "updated"}}
{"action": "search", "query": [0.9, 0.1, 0.0], "k": 10, "ef_search": 64}
{"action": "count"}
{"action": "flush"}
```

Vector Open Config:
```json
{"path": "/path/to/db", "dimension": 512, "metric": "cosine", "max_elements": 100000}
```

### Federation Execute

Request (MessagePack map):
```json
{"action": "add_peer", "name": "alice", "endpoint": "ws://localhost:8080", "status": "active"}
{"action": "get_peer", "id": 1}
{"action": "get_peer_by_name", "name": "alice"}
{"action": "update_peer", "id": 1, "status": "banned"}
{"action": "delete_peer", "id": 1}
{"action": "all_peers"}
{"action": "peer_count"}
{"action": "add_group", "name": "admins", "metadata": {"level": 1}}
{"action": "get_group", "id": 1}
{"action": "get_group_by_name", "name": "admins"}
{"action": "update_group", "id": 1, "metadata": {"level": 2}}
{"action": "delete_group", "id": 1}
{"action": "all_groups"}
{"action": "group_count"}
{"action": "add_member", "group_id": 1, "peer_id": 1}
{"action": "remove_member", "group_id": 1, "peer_id": 1}
{"action": "groups_for_peer", "peer_id": 1}
```

### DAC Execute

Request (MessagePack map):
```json
{"action": "add_rule", "collection": "users", "subject_type": "peer", "subject_id": "alice", "permission": "allow"}
{"action": "add_rule", "collection": "users", "field": "email", "subject_type": "group", "subject_id": "admins", "permission": "redact"}
{"action": "add_rule", "collection": "users", "record_id": "42", "subject_type": "peer", "subject_id": "bob", "permission": "deny", "expires_at": "2025-12-31T23:59:59Z"}
{"action": "get_rule", "id": 1}
{"action": "update_rule", "id": 1, "permission": "deny"}
{"action": "delete_rule", "id": 1}
{"action": "all_rules"}
{"action": "rules_for_collection", "collection": "users"}
{"action": "rule_count"}
{"action": "filter_document", "collection": "users", "document": {"name": "Alice", "email": "a@b.com"}, "peer_id": "alice", "group_ids": ["admins"], "record_id": "42"}
```

### Transport Open Config

```json
{"listen_addr": "0.0.0.0:9400", "mdns_enabled": true, "seed_peers": ["wss://10.0.0.5:9400"], "identity_key": "hex64chars...", "gossip_interval_seconds": 30, "gossip_fan_out": 3, "gossip_ttl": 5, "query_policy": "query_peers_on_miss", "path": "/path/to/audit/db", "nosql_handle": 1, "graph_handle": 2, "vector_handle": 3, "dac_handle": 4, "federation_handle": 5, "mesh_name": "corp-mesh", "mesh_database_name": "warehouse", "mesh_sharing_status": "read_write", "mesh_secret": "optional-hmac-secret", "mesh_max_peers": 16}
```

### Transport Execute

Request (MessagePack map):
```json
{"action": "identity"}
{"action": "connected_peers"}
{"action": "known_peers"}
{"action": "set_credential", "peer_id": "abc123", "token": "bearer-token-value"}
{"action": "connect", "endpoint": "wss://10.0.0.5:9400"}
{"action": "query", "payload": "<binary>", "timeout_secs": 10}
{"action": "audit_log", "offset": 0, "limit": 50}
{"action": "federated_query", "query_type": "nosql", "query_data": {"collection": "users", "action": "find_all"}, "nosql_handle": 1, "timeout_secs": 10, "ttl": 3, "k": 10}  // ttl = max_depth (default 3), visited seeded automatically
{"action": "mesh_status"}
{"action": "mesh_members"}
{"action": "mesh_query", "database": "warehouse", "query_type": "nosql", "query_data": {"collection": "users", "action": "find_all"}, "timeout_secs": 10}
```

### Provenance Execute

Request (MessagePack map):
```json
{"action": "attach", "collection": "users", "record_id": 42, "source_id": "user:alice", "source_type": "user", "content_hash": "abc123...", "user_id": "alice", "is_signed": true, "hops": 0}
{"action": "get", "id": 1}
{"action": "get_for_record", "collection": "users", "record_id": 42}
{"action": "corroborate", "id": 1, "new_source_confidence": 0.70}
{"action": "verify", "id": 1, "public_key": "hex-encoded-ed25519-public-key"}
{"action": "update_confidence", "id": 1, "confidence": 0.90}
{"action": "delete", "id": 1}
{"action": "query", "collection": "users", "source_type": "peer", "verification_status": "verified", "min_confidence": 0.80}
{"action": "count"}
{"action": "compute_hash", "data": {"name": "Alice", "age": 30}}
```

### Key Resolver Execute

Request (MessagePack map):
```json
{"action": "supply_key", "pki_id": "abc123", "user_id": "alice", "public_key_hex": "64-hex-chars...", "trust_level": "explicit", "expires_at_utc": "2026-12-31T23:59:59Z"}
{"action": "get_key", "pki_id": "abc123", "user_id": "alice"}
{"action": "all_keys"}
{"action": "key_count"}
{"action": "revoke_key", "pki_id": "abc123", "user_id": "alice"}
{"action": "delete_key", "id": 1}
{"action": "set_trust_all", "enabled": true}
{"action": "set_trust_all_for_peer", "peer_id": "abc123", "enabled": true}
{"action": "is_trust_all_active"}
{"action": "verify_with_cache", "provenance_handle": 1, "envelope_id": 1}
```

Key Resolver Open Config:
```json
{"path": "/path/to/db"}
```

### AI Provenance Execute

Request (MessagePack map):
```json
{"action": "apply_assessment", "envelope_id": 1, "suggested_confidence": 0.9, "source_type": "user", "reasoning": "...", "tags": {"key": "value"}}
{"action": "apply_conflict_resolution", "envelope_id_a": 1, "envelope_id_b": 2, "confidence_delta_a": -0.1, "confidence_delta_b": 0.05, "preference": "prefer_a", "reasoning": "..."}
{"action": "apply_anomaly_flags", "collection": "users", "flags": [{"record_id": 42, "confidence_penalty": 0.2, "reason": "...", "severity": "high"}]}
{"action": "apply_source_classification", "envelope_id": 1, "source_type": "model", "credibility_prior": 0.5, "reasoning": "..."}
{"action": "get_config"}
```

AI Provenance Open Config:
```json
{"provenance_handle": 1, "ai_blend_weight": 0.3, "enabled_collections": ["users"], "response_timeout_secs": 5, "silent_on_error": true, "rate_limit_per_minute": 60}
```

### AI Query Execute

Request (MessagePack map):
```json
{"action": "process_results", "collection": "products", "results": [{"data": {"name": "Widget", "price": 9.99}, "confidence": 0.92, "source_explanation": "Found via web search", "external_source_uri": "https://example.com", "tags": {"source": "web"}}], "schema": {"required_fields": ["name", "price"], "field_types": {"name": "string", "price": "float"}}}
{"action": "get_config"}
```

AI Query Open Config:
```json
{"nosql_handle": 1, "provenance_handle": 2, "minimum_write_confidence": 0.80, "max_results_per_query": 10, "enabled_collections": ["documents", "products"], "report_write_decisions": true, "rate_limit_per_minute": 20}
```

### Singleton Collections

Request (MessagePack map, via Query endpoint):
```json
{"action": "singleton_create", "collection": "settings", "defaults": {"theme": "dark", "lang": "en"}}
{"action": "singleton_get", "collection": "settings"}
{"action": "is_singleton", "collection": "settings"}
```

Write Transaction (MessagePack array):
```json
[
  {"collection": "settings", "action": "singleton_put", "data": {"theme": "light", "lang": "fr"}},
  {"collection": "settings", "action": "singleton_reset"}
]
```

Singleton collections hold exactly one record (ID=1). `singleton_create` registers the collection with defaults and inserts the initial record. `singleton_put` always upserts ID=1. `singleton_reset` restores defaults. Delete and clear operations are rejected for singleton collections.

### Secure Preferences

Request (MessagePack map, via Query endpoint):
```json
{"action": "pref_get", "store": "app_settings", "key": "theme"}
{"action": "pref_keys", "store": "app_settings"}
{"action": "pref_shareable", "store": "app_settings"}
```

Write Transaction (MessagePack array):
```json
[
  {"collection": "_", "action": "pref_set", "store": "app_settings", "key": "theme", "value": "dark", "shareable": true, "conflict_resolution": "last_write_wins"},
  {"collection": "_", "action": "pref_remove", "store": "app_settings", "key": "theme"}
]
```

Preferences are per-key encrypted key-value stores. When the database has an encryption key (DEK), each preference value is encrypted with an HKDF-derived key unique to that preference key. Conflict resolution strategies: `last_write_wins` (default), `local_wins`, `remote_wins`, `highest_confidence`, `manual`.

### Database Execute (Encryption & Migration)

Request (MessagePack map):
```json
{"action": "owner_key_status"}
{"action": "rotate_owner_key", "current_private_key_hex": "hex64...", "new_private_key_hex": "hex64..."}
{"action": "migrate", "target_version": 2, "operations": [{"type": "rename_tree", "from": "old", "to": "new"}, {"type": "drop_tree", "name": "temp"}]}
```

Open Config with Encryption:
```json
{"path": "/path/to/database", "owner_private_key_hex": "hex64-ed25519-private-key"}
```

## Benchmarks

```bash
cargo bench -p nodedb-ffi             # all benchmarks
```

Benchmark groups: nosql (single_write, get_by_id, filtered_query), graph (add_node, add_edge, traversal_depth3), vector (insert_vector, knn_search_k10), dac (add_rule, evaluate_100_rules), provenance (attach_envelope, compute_hash), keyresolver (register_key, resolve_key), ffi_overhead (open_query_close).

## Cross-Platform Build

```bash
./scripts/build_targets.sh            # build for all installed targets
./scripts/build_targets.sh --release  # release build (default)
```

Requires: `rustup target add <triple>` for each target, `ANDROID_NDK_HOME` for Android targets.

## Tests

```bash
cargo test                            # all tests
cargo test -p nodedb-storage          # storage only
cargo test -p nodedb-nosql            # nosql only
cargo test -p nodedb-graph            # graph only
cargo test -p nodedb-vector           # vector only
cargo test -p nodedb-federation       # federation only
cargo test -p nodedb-dac              # dac only
cargo test -p nodedb-crypto           # crypto only
cargo test -p nodedb-transport        # transport only
cargo test -p nodedb-provenance       # provenance only
cargo test -p nodedb-keyresolver      # keyresolver only
cargo test -p nodedb-ai-provenance    # ai-provenance only
cargo test -p nodedb-ai-query         # ai-query only
cargo test -p nodedb-ffi              # ffi only
```
