/// Configuration for mesh networking identity.
///
/// Defines the mesh network and its cryptographic identity.
/// Pass to [DatabaseMesh.open] or [NodeDB.open] for local-only mode.
class MeshConfig {
  /// Named overlay network (e.g. 'family', 'my-app').
  final String meshName;

  /// Optional HMAC authentication secret for gossip payloads.
  final String? meshSecret;

  /// Optional Ed25519 owner private key (64-character hex string).
  ///
  /// Used for database encryption and signing. If provided, the database
  /// will be encrypted at rest. All databases in a mesh share the same
  /// owner key.
  final String? ownerPrivateKeyHex;

  const MeshConfig({
    required this.meshName,
    this.meshSecret,
    this.ownerPrivateKeyHex,
  });
}
