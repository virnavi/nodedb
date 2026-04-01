import '../util/msgpack.dart';

/// A public key entry in the NodeDB key resolver.
class KeyEntry {
  final int id;
  final String pkiId;
  final String userId;
  final String publicKeyHex;
  final String trustLevel;
  final DateTime? expiresAtUtc;
  final DateTime createdAtUtc;

  const KeyEntry({
    required this.id,
    required this.pkiId,
    required this.userId,
    required this.publicKeyHex,
    required this.trustLevel,
    this.expiresAtUtc,
    required this.createdAtUtc,
  });

  factory KeyEntry.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final pkiId = (decodeField(decoded, 'pki_id', 1) ?? '') as String;
    final userId = (decodeField(decoded, 'user_id', 2) ?? '') as String;
    final publicKeyHex =
        (decodeField(decoded, 'public_key_hex', 3) ?? '') as String;
    final trustLevel =
        _decodeTrustLevel(decodeField(decoded, 'trust_level', 4));
    final expiresStr = decodeField(decoded, 'expires_at_utc', 5);
    final expiresAtUtc =
        expiresStr is String ? DateTime.tryParse(expiresStr) : null;
    final createdStr = decodeField(decoded, 'cached_at_utc', 6) ?? decodeField(decoded, 'created_at_utc', 6);
    final createdAtUtc = createdStr is String
        ? (DateTime.tryParse(createdStr) ??
            DateTime.fromMillisecondsSinceEpoch(0))
        : DateTime.fromMillisecondsSinceEpoch(0);

    return KeyEntry(
      id: id,
      pkiId: pkiId,
      userId: userId,
      publicKeyHex: publicKeyHex,
      trustLevel: trustLevel,
      expiresAtUtc: expiresAtUtc,
      createdAtUtc: createdAtUtc,
    );
  }

  /// Decode KeyTrustLevel from string or positional enum [variant_idx, []].
  static String _decodeTrustLevel(dynamic value) {
    if (value is String) return value;
    if (value is List && value.isNotEmpty) {
      final idx = value[0];
      const levels = ['explicit', 'trust_all', 'revoked'];
      if (idx is int && idx >= 0 && idx < levels.length) return levels[idx];
    }
    return 'explicit';
  }

  @override
  String toString() =>
      'KeyEntry(id: $id, pkiId: $pkiId, userId: $userId, trust: $trustLevel)';
}
