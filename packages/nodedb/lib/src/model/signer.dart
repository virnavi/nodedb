import '../engine/nosql_engine.dart';
import '../engine/provenance_engine.dart';
import 'keypair.dart';
import 'provenance_envelope.dart';

/// Signs provenance data using Ed25519 via the Rust FFI layer.
///
/// Wraps the "compute_hash" and "sign" FFI actions to produce
/// provenance-compatible signatures without a Dart crypto dependency.
class NodeDBSigner {
  final NoSqlEngine _nosql;
  final ProvenanceEngine _provenance;
  final NodeDBKeyPair _keypair;
  final String _userId;

  NodeDBSigner({
    required NoSqlEngine nosql,
    required ProvenanceEngine provenance,
    required NodeDBKeyPair keypair,
    required String userId,
  })  : _nosql = nosql,
        _provenance = provenance,
        _keypair = keypair,
        _userId = userId;

  /// The public key hex (used as pkiId in provenance).
  String get pkiId => _keypair.publicKeyHex;

  /// The user ID.
  String get userId => _userId;

  /// Compute a content hash for the given data via the provenance engine.
  String computeHash(dynamic data) => _provenance.computeHash(data);

  /// Sign a provenance payload.
  ///
  /// Builds the canonical payload format:
  ///   `{contentHash}|{createdAtUtc}|{pkiId}|{userId}`
  /// then signs it with the Ed25519 private key via FFI.
  ///
  /// Returns the hex-encoded Ed25519 signature (128 chars).
  String signPayload({
    required String contentHash,
    required String createdAtUtc,
  }) {
    final payload = '$contentHash|$createdAtUtc|$pkiId|$_userId';
    return _nosql.signData(_keypair.privateKeyHex, payload);
  }

  /// Attach signed provenance to a record.
  ///
  /// Computes the content hash, signs the provenance payload, and
  /// attaches the envelope with signature fields populated.
  ProvenanceEnvelope attachSigned({
    required String collection,
    required int recordId,
    required dynamic data,
    required String sourceId,
    String sourceType = 'user',
  }) {
    final contentHash = computeHash(data);
    final createdAtUtc = DateTime.now().toUtc().toIso8601String();
    final signature = signPayload(
      contentHash: contentHash,
      createdAtUtc: createdAtUtc,
    );

    return _provenance.attach(
      collection: collection,
      recordId: recordId,
      sourceId: sourceId,
      sourceType: sourceType,
      contentHash: contentHash,
      pkiSignature: signature,
      pkiId: pkiId,
      userId: _userId,
      isSigned: true,
      createdAtUtc: createdAtUtc,
    );
  }
}
