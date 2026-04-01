# FFI Protocol

[← Back to Index](README.md)

The FFI layer bridges Rust and Dart via a C-compatible interface using MessagePack serialization.

## Overview

```
Dart (nodedb_ffi)                    Rust (nodedb-ffi)
─────────────────                    ─────────────────
msgpackEncode(request)
  → ffi.execute(handle, bytes) ──→   deserialize request
                                      dispatch to engine
                                      serialize response
  ← bytes ─────────────────────────  return msgpack bytes
msgpackDecode(response)
```

All data crosses the FFI boundary as MessagePack-encoded byte arrays. Engines are referenced by `u64` handles stored in Rust-side `RwLock<HashMap>`.

## Shared Library

The `nodedb-ffi` crate compiles as:
- `libnodedb_ffi.dylib` (macOS)
- `libnodedb_ffi.so` (Linux/Android)
- `libnodedb_ffi.dll` (Windows)
- `libnodedb_ffi.a` (static)

## Function Signatures

### Open/Close Pattern

Every engine follows the same pattern:

```c
// Open — returns handle via out parameter
bool nodedb_<engine>_open(
    const uint8_t* config_ptr,  // MessagePack config
    size_t config_len,
    uint64_t* out_handle,       // Output handle
    NodeDbError* out_error      // Output error
) -> bool;

// Execute — request/response via MessagePack
bool nodedb_<engine>_execute(
    uint64_t handle,
    const uint8_t* request_ptr, // MessagePack request
    size_t request_len,
    uint8_t** out_response,     // Output response bytes
    size_t* out_response_len,
    NodeDbError* out_error
) -> bool;

// Close
void nodedb_<engine>_close(uint64_t handle);
```

### Available Engines

| Engine | Open Function | Execute Function |
|--------|--------------|-----------------|
| NoSQL | `nodedb_db_open` | `nodedb_db_execute` |
| Graph | `nodedb_graph_open` | `nodedb_graph_execute` |
| Vector | `nodedb_vector_open` | `nodedb_vector_execute` |
| Federation | `nodedb_federation_open` | `nodedb_federation_execute` |
| DAC | `nodedb_dac_open` | `nodedb_dac_execute` |
| Transport | `nodedb_transport_open` | `nodedb_transport_execute` |
| Provenance | `nodedb_provenance_open` | `nodedb_provenance_execute` |
| KeyResolver | `nodedb_keyresolver_open` | `nodedb_keyresolver_execute` |
| AI Provenance | `nodedb_ai_provenance_open` | `nodedb_ai_provenance_execute` |
| AI Query | `nodedb_ai_query_open` | `nodedb_ai_query_execute` |

### Special Functions

```c
// Write transaction (NoSQL)
bool nodedb_write_txn(uint64_t handle, const uint8_t* ops, size_t len,
                      uint8_t** out, size_t* out_len, NodeDbError* err);

// Link transport to NoSQL (for mesh triggers)
bool nodedb_link_transport(uint64_t db_handle, uint64_t transport_handle,
                           NodeDbError* err);

// Free memory
void nodedb_free_buffer(uint8_t* ptr, size_t len);
void nodedb_free_error(NodeDbError* err);

// Version
int nodedb_ffi_version();
```

## Error Codes

### General Errors (0–9)

| Code | Constant | Description |
|------|----------|-------------|
| 0 | `ERR_NONE` | No error |
| 1 | `ERR_INVALID_HANDLE` | Handle not found in map |
| 2 | `ERR_STORAGE` | Sled storage error |
| 3 | `ERR_SERIALIZATION` | MessagePack encode/decode error |
| 4 | `ERR_NOT_FOUND` | Record not found |
| 5 | `ERR_INVALID_QUERY` | Malformed query or missing action |
| 6 | `ERR_INTERNAL` | Panic or unexpected error |
| 7 | `ERR_NULL_POINTER` | Null pointer argument |

### Graph Errors (10–13)

| Code | Constant | Description |
|------|----------|-------------|
| 10 | `ERR_GRAPH_NODE_NOT_FOUND` | Node ID doesn't exist |
| 11 | `ERR_GRAPH_EDGE_NOT_FOUND` | Edge ID doesn't exist |
| 12 | `ERR_GRAPH_DELETE_RESTRICTED` | Delete restricted by edge |
| 13 | `ERR_GRAPH_TRAVERSAL` | Traversal algorithm error |

### Vector Errors (20–23)

| Code | Constant | Description |
|------|----------|-------------|
| 20 | `ERR_VECTOR_NOT_FOUND` | Vector ID not found |
| 21 | `ERR_VECTOR_DIMENSION_MISMATCH` | Wrong vector dimensionality |
| 22 | `ERR_VECTOR_NOT_INITIALIZED` | Engine not initialized |
| 23 | `ERR_VECTOR_SEARCH` | Search algorithm error |

### Federation Errors (30–33)

| Code | Constant | Description |
|------|----------|-------------|
| 30 | `ERR_FEDERATION_PEER_NOT_FOUND` | Peer not found |
| 31 | `ERR_FEDERATION_GROUP_NOT_FOUND` | Group not found |
| 32 | `ERR_FEDERATION_DUPLICATE_NAME` | Name already exists |
| 33 | `ERR_FEDERATION_INVALID_MEMBER` | Invalid group member |

### DAC Errors (40–42)

| Code | Constant | Description |
|------|----------|-------------|
| 40 | `ERR_DAC_RULE_NOT_FOUND` | Rule not found |
| 41 | `ERR_DAC_INVALID_COLLECTION` | Invalid collection |
| 42 | `ERR_DAC_INVALID_DOCUMENT` | Invalid document format |

### Transport Errors (50–55)

| Code | Constant | Description |
|------|----------|-------------|
| 50 | `ERR_TRANSPORT_CONNECTION` | Connection/TLS/WebSocket error |
| 51 | `ERR_TRANSPORT_HANDSHAKE` | Handshake failure |
| 52 | `ERR_TRANSPORT_SEND` | Send/receive error |
| 53 | `ERR_TRANSPORT_TIMEOUT` | Operation timeout |
| 54 | `ERR_TRANSPORT_PEER_REJECTED` | Peer rejected by credential check |
| 55 | `ERR_TRANSPORT_CRYPTO` | Cryptographic error |

### Provenance Errors (60–63)

| Code | Constant | Description |
|------|----------|-------------|
| 60 | `ERR_PROVENANCE_NOT_FOUND` | Envelope not found |
| 61 | `ERR_PROVENANCE_INVALID_CONFIDENCE` | Confidence out of range |
| 62 | `ERR_PROVENANCE_VERIFICATION` | Verification error |
| 63 | `ERR_PROVENANCE_CANONICAL` | Canonical form error |

### Key Resolver Errors (70–73)

| Code | Constant | Description |
|------|----------|-------------|
| 70 | `ERR_KEYRESOLVER_NOT_FOUND` | Key not found |
| 71 | `ERR_KEYRESOLVER_INVALID_HEX` | Invalid hex string |
| 72 | `ERR_KEYRESOLVER_EXPIRED` | Key has expired |
| 73 | `ERR_KEYRESOLVER_ENTRY_NOT_FOUND` | Entry not found |

### AI Provenance Errors (80–83)

| Code | Constant | Description |
|------|----------|-------------|
| 80 | `ERR_AI_PROVENANCE_ENVELOPE_NOT_FOUND` | Envelope not found |
| 81 | `ERR_AI_PROVENANCE_INVALID_CONFIDENCE` | Invalid confidence value |
| 82 | `ERR_AI_PROVENANCE_COLLECTION_NOT_ENABLED` | Collection not in enabled list |
| 83 | `ERR_AI_PROVENANCE_CONFIG` | Configuration error |

### AI Query Errors (90–94)

| Code | Constant | Description |
|------|----------|-------------|
| 90 | `ERR_AI_QUERY_SCHEMA_VALIDATION` | Schema validation failed |
| 91 | `ERR_AI_QUERY_CONFIDENCE_BELOW_THRESHOLD` | Confidence too low |
| 92 | `ERR_AI_QUERY_COLLECTION_NOT_ENABLED` | Collection not enabled |
| 93 | `ERR_AI_QUERY_CONFIG` | Configuration error |
| 94 | `ERR_AI_QUERY_NOSQL` | NoSQL engine error |

### Trigger/Singleton/Preference Errors (100–135)

| Code | Constant | Description |
|------|----------|-------------|
| 100 | `ERR_TRIGGER_ABORT` | Trigger aborted the operation |
| 101 | `ERR_TRIGGER_NOT_FOUND` | Trigger not found |
| 110 | `ERR_SINGLETON_DELETE` | Cannot delete singleton |
| 111 | `ERR_SINGLETON_CLEAR` | Cannot clear singleton collection |
| 120 | `ERR_PREFERENCE_NOT_FOUND` | Preference key not found |
| 121 | `ERR_PREFERENCE_ERROR` | Preference operation error |
| 130 | `ERR_RESERVED_SCHEMA_WRITE` | Cannot write to reserved schema |
| 140 | `ERR_ACCESS_HISTORY` | Access history error |
| 141 | `ERR_TRIM_NEVER_TRIM` | Record marked as never-trim |
| 142 | `ERR_TRIM_POLICY_INVALID` | Invalid trim policy |
| 143 | `ERR_TRIM_ABORTED` | Trim operation aborted |

### Pairing Errors (150–153)

| Code | Constant | Description |
|------|----------|-------------|
| 150 | `ERR_PAIRING_REQUIRED` | Peer needs user approval |
| 151 | `ERR_PAIRING_VERIFICATION_FAILED` | Stored key/user_id mismatch |
| 152 | `ERR_PAIRING_NOT_FOUND` | Pending pairing not found |
| 153 | `ERR_PAIRING_ERROR` | General pairing error |

## Request/Response Format

All execute requests are MessagePack maps with an `action` key:

```dart
// Request
{'action': 'query', 'collection': 'public.users', 'query': {...}}

// Response (success)
{'documents': [...], 'count': 5}

// Error — set via NodeDbError struct, function returns false
```

### Common Transport Actions

| Action | Request Fields | Response |
|--------|---------------|----------|
| `identity` | — | `{peer_id, public_key_bytes}` |
| `connected_peers` | — | `{count, peer_ids}` |
| `known_peers` | — | `{peers: [{peer_id, endpoint, status, ttl}]}` |
| `connect` | `endpoint` | `{peer_id}` |
| `mesh_status` | — | `{mesh_name, database_name, ...}` |
| `mesh_members` | — | `{members: [...]}` |
| `federated_query` | `query_type, query_data, timeout_secs, ttl` | `{results: [...]}` |
| `paired_devices` | — | `{devices: [{peer_id, user_id, device_name, ...}]}` |
| `pending_pairings` | — | `{pending: [{peer_id, user_id, device_name, ...}]}` |
| `approve_pairing` | `peer_id` | `{ok, peer_id, user_id}` |
| `reject_pairing` | `peer_id` | `{removed}` |
| `remove_paired_device` | `peer_id` | `{removed}` |
| `audit_log` | `limit?` | `[{event, peer_id, timestamp}]` |

## Dart FFI Bindings

The `nodedb_ffi` package wraps all FFI calls:

```dart
class NodeDbBindings {
  // NoSQL
  late final Pointer<NativeFunction<...>> open;
  late final Pointer<NativeFunction<...>> close;
  late final Pointer<NativeFunction<...>> dbExecute;
  late final Pointer<NativeFunction<...>> writeTxn;
  late final Pointer<NativeFunction<...>> query;

  // Graph, Vector, Federation, DAC, Transport,
  // Provenance, KeyResolver, AI Provenance, AI Query
  // ... same pattern for each engine
}
```

Helper functions:
- `openRaw(bindings, openFn, configBytes)` → handle
- `executeRaw(bindings, executeFn, handle, requestBytes)` → response bytes
- `msgpackEncode(Map)` → `Uint8List`
- `msgpackDecode(Uint8List)` → `dynamic`

## Related Pages

- [Rust Crates](rust-crates.md) — `nodedb-ffi` crate details
- [Dart Packages](dart-packages.md) — `nodedb_ffi` package details
- [Architecture](architecture.md) — overall system design
