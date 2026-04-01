/// An Ed25519 keypair for database ownership and signing.
class NodeDBKeyPair {
  /// 64-char hex-encoded Ed25519 private (signing) key.
  final String privateKeyHex;

  /// 64-char hex-encoded Ed25519 public (verifying) key.
  final String publicKeyHex;

  const NodeDBKeyPair({
    required this.privateKeyHex,
    required this.publicKeyHex,
  });

  @override
  String toString() =>
      'NodeDBKeyPair(publicKeyHex: ${publicKeyHex.substring(0, 8)}...)';
}

/// Status of the owner key binding for a database.
enum OwnerKeyStatus {
  /// The database was opened with the correct owner key.
  verified,

  /// The database was opened with a key that does not match.
  mismatch,

  /// The database has no owner key bound.
  unbound;

  static OwnerKeyStatus fromString(String s) {
    switch (s) {
      case 'verified':
        return OwnerKeyStatus.verified;
      case 'mismatch':
        return OwnerKeyStatus.mismatch;
      case 'unbound':
      default:
        return OwnerKeyStatus.unbound;
    }
  }
}

/// Result of a key rotation.
class RotateKeyResult {
  final String status;
  final String newFingerprint;

  const RotateKeyResult({
    required this.status,
    required this.newFingerprint,
  });

  @override
  String toString() =>
      'RotateKeyResult(status: $status, fingerprint: ${newFingerprint.substring(0, 8)}...)';
}
