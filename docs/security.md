# Security

[← Back to Index](README.md)

NodeDB provides multiple layers of security: encryption at rest, cryptographic identity, access control, and key management.

## Encryption at Rest

### Database Encryption Key (DEK)

Every database can be encrypted with AES-256-GCM:

- **DEK**: A random 256-bit key generated on first open
- **Key wrapping**: The DEK is sealed with the owner's Ed25519-derived X25519 key
- **Sealed DEK** is stored in the `DbHeader` (persisted in sled)
- On open: unseal DEK with owner key → decrypt all reads, encrypt all writes

```dart
// Open with encryption (owner key binding)
final db = NodeDB.open(
  directory: '/path/to/data',
  ownerPrivateKeyHex: '...', // 64-char hex Ed25519 signing key
);
```

### Per-Preference Encryption

Preferences use per-key HKDF-derived encryption:

```
derivedKey = HKDF-SHA256(dek, "prefs:" + keyName)
ciphertext = AES-256-GCM(derivedKey, plaintext)
```

Each preference key gets its own derived encryption key, so compromising one key doesn't expose others.

### Storage Modes

| Mode | Description |
|------|-------------|
| Normal | DEK matches owner key — full read/write |
| Encrypted (mismatch) | DEK exists but owner key doesn't match — reads return empty, writes are no-ops |
| Unencrypted | No owner key — plain msgpack storage |

## Cryptographic Identity

### Ed25519 Key Pair

Every NodeDB transport instance has an Ed25519 identity:

```dart
// Generate a new identity
final keypair = db.nosql.generateKeypair();
// {private_key_hex: '...', public_key_hex: '...'}

// Or provide a stable identity
const TransportConfig(
  identityKeyHex: '64-char-hex-ed25519-signing-key',
)
```

### NodeIdentity (Rust)

```rust
pub struct NodeIdentity {
    signing_key: ed25519_dalek::SigningKey,   // Ed25519
    verifying_key: ed25519_dalek::VerifyingKey,
}

impl NodeIdentity {
    pub fn generate() -> Self;
    pub fn from_signing_key_bytes(bytes: &[u8; 32]) -> Result<Self>;
    pub fn peer_id(&self) -> String;            // Hex of public key
    pub fn verifying_key_bytes(&self) -> Vec<u8>;
    pub fn sign(&self, message: &[u8]) -> Vec<u8>;
}
```

### PublicIdentity

```rust
pub struct PublicIdentity {
    pub peer_id: String,           // 64-char hex
    pub public_key_bytes: Vec<u8>, // 32 bytes
}
```

## Transport Security

### TLS

All WebSocket connections use TLS:

- **Certificate generation**: Self-signed certs via `rcgen` (generated at startup)
- **Protocol**: TLS 1.3 via `rustls`
- **Client verification**: Optional (via trusted_peer_keys or pairing)

### Handshake Authentication

The Hello/HelloAck handshake includes:

1. **Nonce exchange** — random bytes to prevent replay attacks
2. **Signature verification** — Ed25519 signature over the nonce proves key ownership
3. **Credential check** — via trusted key whitelist, pairing store, or callback

### Peer Trust Models

| Model | Configuration | Behavior |
|-------|--------------|----------|
| Accept all | `trustedPeerKeys: []`, `requirePairing: false` | Any peer accepted |
| Whitelist | `trustedPeerKeys: ['key1', 'key2']` | Only listed keys accepted |
| Pairing | `requirePairing: true` | User must approve each new peer |

## Discretionary Access Control (DAC)

The DAC engine provides fine-grained access control:

```dart
final dac = db.dac!;

// Collection-level rule
dac.addRule(
  collection: 'public.medical_records',
  subjectType: 'peer',
  subjectId: 'peer-uuid',
  permission: 'deny',
);

// Field-level redaction
dac.addRule(
  collection: 'public.users',
  field: 'ssn',
  subjectType: 'group',
  subjectId: 'external-group-id',
  permission: 'redact',
);

// Record-level access
dac.addRule(
  collection: 'public.documents',
  recordId: 42,
  subjectType: 'peer',
  subjectId: 'peer-uuid',
  permission: 'allow',
  expiresAt: DateTime.now().add(Duration(days: 30)),
);

// Filter a document based on subject's permissions
final filtered = dac.filterDocument(document, subjectType: 'peer', subjectId: 'peer-uuid');
```

### Permission Types

| Permission | Effect |
|-----------|--------|
| `allow` | Grant access to the resource |
| `deny` | Deny access entirely |
| `redact` | Allow access but redact specific fields |

### Rule Hierarchy

Rules are evaluated from most specific to least specific:

1. Record-level rules (specific document)
2. Field-level rules (specific field in collection)
3. Collection-level rules (entire collection)

## Key Management

### Key Resolver

The key resolver stores and manages public keys for signature verification:

```dart
final keyResolver = db.keyResolver!;

// Register a public key
keyResolver.addKey(
  peerId: 'peer-uuid',
  publicKeyHex: '64-char-hex',
  trustLevel: 'explicit',
);

// Resolve a key
final entry = keyResolver.resolvePublicKey('peer-uuid');

// Verify a signature
final valid = keyResolver.verifySignature(
  pubKeyHex: '...',
  signature: signatureBytes,
  payload: payloadBytes,
);

// Trust management
keyResolver.setTrustAll('peer-uuid', true);  // Trust all keys from this peer
keyResolver.revoke('peer-uuid');              // Revoke trust
```

### Trust Levels

| Level | Description |
|-------|-------------|
| `explicit` | Manually verified and trusted |
| `trust_all` | Accept all keys from this source (in-memory only) |
| `revoked` | Previously trusted, now revoked |

### Key Entry

```dart
class KeyEntry {
  final String pkiId;           // Key identifier
  final String userId;          // Associated user
  final String publicKeyHex;    // Ed25519 public key
  final String trustLevel;      // explicit, trust_all, revoked
  final DateTime? expiresAtUtc; // Optional expiration
  final DateTime cachedAtUtc;   // When stored
}
```

## Signature Verification Flow

```
1. Record written → compute content hash (SHA-256)
2. Build signature payload: "{hash}|{timestamp}|{pkiId}|{userId}"
3. Sign with Ed25519: signature = identity.sign(payload)
4. Store in ProvenanceEnvelope: {signature, pkiId, contentHash, ...}

Verification:
1. Fetch envelope for record
2. Resolve public key via KeyResolver
3. Rebuild payload from envelope fields
4. ed25519_verify(signature, payload, publicKey)
5. Update verificationStatus: verified/failed
```

## Rust Implementation

**Crates**:
- `nodedb-crypto` — Ed25519, X25519, AES-256-GCM, HKDF, sealed envelopes
- `nodedb-dac` — access control rules and filtering
- `nodedb-keyresolver` — public key registry and trust management

## Related Pages

- [Data Provenance](provenance.md) — signature-based verification
- [Transport Layer](transport.md) — TLS, handshake, pairing
- [Federation & Mesh](federation.md) — mesh secrets, peer authentication
