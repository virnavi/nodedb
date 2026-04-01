# Data Provenance

[← Back to Index](README.md)

The provenance system tracks the origin, confidence, and verification status of every record in the database.

## Overview

Each document can have one or more `ProvenanceEnvelope` attached, recording:
- **Who** created/modified the data (source ID, user ID)
- **How** the data arrived (source type: peer, import, user, AI, sensor)
- **When** it was created and last verified
- **How confident** the system is in the data (0.0–1.0)
- **Whether** the data's integrity has been cryptographically verified

## Enable Provenance

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  provenanceEnabled: true,
);
```

## ProvenanceEnvelope

The envelope has 28 fields organized in four groups:

### Core Fields (indices 0–12)

| Field | Type | Description |
|-------|------|-------------|
| `id` | int | Envelope ID |
| `collection` | String | Collection name |
| `recordId` | int | Document ID |
| `confidenceFactor` | double | Confidence score (0.0–1.0) |
| `sourceId` | String | Source identifier |
| `sourceType` | String | Source category (see below) |
| `contentHash` | String | SHA-256 of record data |
| `createdAtUtc` | DateTime | When envelope was created |
| `updatedAtUtc` | DateTime | Last modification |
| `pkiSignature` | String? | Ed25519 signature bytes (hex) |
| `pkiId` | String? | Signing key identifier |
| `userId` | String? | User who created the data |
| `verificationStatus` | String | Current verification state |

### AI Augmentation Fields (indices 13–19)

| Field | Type | Description |
|-------|------|-------------|
| `aiAugmented` | bool | Whether AI has assessed this envelope |
| `aiRawConfidence` | double? | AI's suggested confidence |
| `aiBlendWeightUsed` | double? | Weight used in blending |
| `aiReasoning` | String? | AI's explanation |
| `aiTags` | Map? | AI-generated tags |
| `aiAnomalyFlagged` | bool | Whether AI flagged anomaly |
| `aiAnomalySeverity` | String? | low, medium, high |

### AI Origin Fields (indices 20–23)

| Field | Type | Description |
|-------|------|-------------|
| `aiOriginated` | bool | Whether data came from AI |
| `aiOriginTag` | String? | e.g., `ai-query:products:2026-03-15T10:00:00Z` |
| `aiSourceExplanation` | String? | How AI generated the data |
| `aiExternalSourceUri` | String? | External data source URL |

### Lifecycle Fields (indices 24–27)

| Field | Type | Description |
|-------|------|-------------|
| `checkedAtUtc` | DateTime? | Last integrity check |
| `dataUpdatedAtUtc` | DateTime? | When underlying data changed |
| `localId` | String? | Local identifier |
| `globalId` | String? | Global identifier |

## Source Types

| Type | Initial Confidence | Description |
|------|-------------------|-------------|
| `user` | 0.90 | Direct user input |
| `peer` | 0.75 | From a federated peer |
| `import` | 0.70 | Bulk import |
| `model` | 0.65 | Machine learning model |
| `sensor` | 0.60 | IoT sensor data |
| `ai_query` | 0.50 | AI-generated query result |
| `unknown` | 0.50 | Unknown origin |

## Verification Status

| Status | Meaning |
|--------|---------|
| `unverified` | No verification attempted |
| `verified` | Signature verified successfully |
| `failed` | Signature verification failed |
| `key_requested` | Waiting for public key |
| `trust_all` | Peer is trust-all (skip verification) |

## Operations

### Attach Provenance

```dart
final envelope = db.provenance!.attach(
  collection: 'public.users',
  recordId: doc.id,
  sourceId: 'device-a',
  sourceType: 'user',
  contentHash: computedHash,
  userId: 'user-uuid',
);
```

### Query Envelopes

```dart
// Get envelope by ID
final env = db.provenance!.get(envelopeId);

// Get all envelopes for a record
final envelopes = db.provenance!.getForRecord('public.users', recordId);

// Query with filters
final results = db.provenance!.query(
  collection: 'public.users',
  sourceType: 'peer',
  verificationStatus: 'verified',
  minConfidence: 0.7,
);
```

### Confidence Management

```dart
// Corroborate — boost confidence via multiple sources
db.provenance!.corroborate(envelopeId, 0.85);

// Manual update
db.provenance!.updateConfidence(envelopeId, 0.6);
```

#### Confidence Functions

| Function | Formula | Description |
|----------|---------|-------------|
| `initial_confidence` | Based on source type | Starting confidence |
| `corroborate` | `c + (1 - c) * 0.1` | Boost from multiple sources |
| `conflict` | `c * 0.85` | Penalty for conflicting data |
| `verification_boost` | `c + 0.10` (clamped) | Boost after successful verification |
| `verification_failure` | `c * 0.5` | Penalty for failed verification |
| `age_decay` | `c * 0.5^(days/halfLife)` | Decay over time |

### Verification

```dart
// Verify with cached key
final verified = db.provenance!.verify(envelopeId, publicKeyHex);

// Cross-engine verification (keyresolver + provenance)
final result = db.verifyWithCache(envelopeId);
```

Verification checks:
1. Rebuild signature payload: `"{contentHash}|{createdAtUtc}|{pkiId}|{userId}"`
2. Ed25519 verify signature against public key
3. Update `verificationStatus` on the envelope

## With Code Generation

Use `@ProvenanceConfig` annotation:

```dart
@collection
@ProvenanceConfig(confidenceDecayHalfLifeDays: 30)
class SensorReading {
  double temperature;
  DateTime timestamp;
  // ...
}
```

Generated DAO includes:
```dart
List<WithProvenance<SensorReading>> findAllWithProvenance();
```

## FindAllWithProvenance

```dart
// Returns documents paired with their latest provenance envelope
final results = db.findAllWithProvenance('public.sensor_readings');

for (final r in results) {
  final doc = r.data;
  final prov = r.provenance;
  print('${doc.data['temperature']} — confidence: ${prov?.confidenceFactor}');
}
```

## Rust Implementation

**Crate**: `nodedb-provenance` → depends on `nodedb-storage`, `nodedb-crypto`

Key types:
- `ProvenanceEngine` — envelope CRUD and confidence management
- `ProvenanceEnvelope` — 24-field metadata record (+ 4 lifecycle)
- `ProvenanceSourceType` — source category enum
- `ProvenanceVerificationStatus` — verification state enum

Key functions:
- `compute_content_hash()` — SHA-256 of canonical msgpack
- `canonical_msgpack()` — deterministic serialization
- `build_signature_payload()` — construct signable string
- `verify_signature()` — Ed25519 verification

## Related Pages

- [AI Integration](ai-integration.md) — AI confidence assessment and blending
- [Security](security.md) — Ed25519 signatures, key management
- [Query System](query-system.md) — `withProvenance()` query flag
