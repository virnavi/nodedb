import '../util/msgpack.dart';

/// A provenance envelope tracking the origin and trust of a record.
class ProvenanceEnvelope {
  final int id;
  final String collection;
  final int recordId;
  final double confidenceFactor;
  final String sourceId;
  final String sourceType;
  final String contentHash;
  final DateTime createdAtUtc;
  final DateTime updatedAtUtc;
  final String? pkiSignature;
  final String? pkiId;
  final String? userId;
  final String verificationStatus;
  // AI augmentation fields (indices 13+)
  final bool aiAugmented;
  final double? aiRawConfidence;
  final double? aiBlendWeightUsed;
  final String? aiReasoning;
  final Map<String, dynamic>? aiTags;
  final bool aiAnomalyFlagged;
  final String? aiAnomalySeverity;
  // AI origin fields (indices 20+)
  final bool aiOriginated;
  final String? aiOriginTag;
  final String? aiSourceExplanation;
  final String? aiExternalSourceUri;
  // Timestamp tracking (index 24)
  final DateTime? checkedAtUtc;
  // Lifecycle fields (indices 25-27)
  final DateTime? dataUpdatedAtUtc;
  final String? localId;
  final String? globalId;

  const ProvenanceEnvelope({
    required this.id,
    required this.collection,
    required this.recordId,
    required this.confidenceFactor,
    required this.sourceId,
    required this.sourceType,
    required this.contentHash,
    required this.createdAtUtc,
    required this.updatedAtUtc,
    this.pkiSignature,
    this.pkiId,
    this.userId,
    required this.verificationStatus,
    this.aiAugmented = false,
    this.aiRawConfidence,
    this.aiBlendWeightUsed,
    this.aiReasoning,
    this.aiTags,
    this.aiAnomalyFlagged = false,
    this.aiAnomalySeverity,
    this.aiOriginated = false,
    this.aiOriginTag,
    this.aiSourceExplanation,
    this.aiExternalSourceUri,
    this.checkedAtUtc,
    this.dataUpdatedAtUtc,
    this.localId,
    this.globalId,
  });

  factory ProvenanceEnvelope.fromMsgpack(dynamic decoded) {
    return ProvenanceEnvelope(
      id: decodeField(decoded, 'id', 0) as int,
      collection: (decodeField(decoded, 'collection', 1) ?? '') as String,
      recordId: decodeField(decoded, 'record_id', 2) as int,
      confidenceFactor:
          (decodeField(decoded, 'confidence_factor', 3) as num).toDouble(),
      sourceId: (decodeField(decoded, 'source_id', 4) ?? '') as String,
      sourceType: _decodeSourceType(decodeField(decoded, 'source_type', 5)),
      contentHash: (decodeField(decoded, 'content_hash', 6) ?? '') as String,
      createdAtUtc: _parseDateTime(decodeField(decoded, 'created_at_utc', 7)),
      updatedAtUtc: _parseDateTime(decodeField(decoded, 'updated_at_utc', 8)),
      pkiSignature: decodeField(decoded, 'pki_signature', 9) as String?,
      pkiId: decodeField(decoded, 'pki_id', 10) as String?,
      userId: decodeField(decoded, 'user_id', 11) as String?,
      verificationStatus:
          _decodeVerificationStatus(decodeField(decoded, 'verification_status', 12)),
      aiAugmented: decodeField(decoded, 'ai_augmented', 13) == true,
      aiRawConfidence: _toNullableDouble(decodeField(decoded, 'ai_raw_confidence', 14)),
      aiBlendWeightUsed:
          _toNullableDouble(decodeField(decoded, 'ai_blend_weight_used', 15)),
      aiReasoning: decodeField(decoded, 'ai_reasoning', 16) as String?,
      aiTags: _toNullableMap(decodeField(decoded, 'ai_tags', 17)),
      aiAnomalyFlagged: decodeField(decoded, 'ai_anomaly_flagged', 18) == true,
      aiAnomalySeverity:
          decodeField(decoded, 'ai_anomaly_severity', 19) as String?,
      aiOriginated: decodeField(decoded, 'ai_originated', 20) == true,
      aiOriginTag: decodeField(decoded, 'ai_origin_tag', 21) as String?,
      aiSourceExplanation:
          decodeField(decoded, 'ai_source_explanation', 22) as String?,
      aiExternalSourceUri:
          decodeField(decoded, 'ai_external_source_uri', 23) as String?,
      checkedAtUtc: _parseNullableDateTime(
          decodeField(decoded, 'checked_at_utc', 24)),
      dataUpdatedAtUtc: _parseNullableDateTime(
          decodeField(decoded, 'data_updated_at_utc', 25)),
      localId: decodeField(decoded, 'local_id', 26) as String?,
      globalId: decodeField(decoded, 'global_id', 27) as String?,
    );
  }

  static DateTime? _parseNullableDateTime(dynamic value) {
    if (value == null) return null;
    if (value is String) return DateTime.tryParse(value);
    return null;
  }

  static DateTime _parseDateTime(dynamic value) {
    if (value is String) {
      return DateTime.tryParse(value) ?? DateTime.fromMillisecondsSinceEpoch(0);
    }
    return DateTime.fromMillisecondsSinceEpoch(0);
  }

  static double? _toNullableDouble(dynamic value) {
    if (value == null) return null;
    if (value is double) return value;
    if (value is num) return value.toDouble();
    return null;
  }

  static Map<String, dynamic>? _toNullableMap(dynamic value) {
    if (value is Map) return Map<String, dynamic>.from(value);
    return null;
  }

  /// Decode source type from positional enum or string.
  static String _decodeSourceType(dynamic value) {
    if (value is String) return value;
    // rmpv::ext::to_value encodes enums as Array([variant_idx, Array([])])
    if (value is List && value.isNotEmpty) {
      final idx = value[0];
      const types = [
        'peer', 'import', 'model', 'user', 'sensor', 'ai_query', 'unknown',
      ];
      if (idx is int && idx >= 0 && idx < types.length) return types[idx];
    }
    return 'unknown';
  }

  /// Decode verification status from positional enum or string.
  static String _decodeVerificationStatus(dynamic value) {
    if (value is String) return value;
    // rmpv::ext::to_value encodes enums as Array([variant_idx, Array([])])
    if (value is List && value.isNotEmpty) {
      final idx = value[0];
      const statuses = [
        'unverified',
        'verified',
        'failed',
        'key_requested',
        'trust_all',
      ];
      if (idx is int && idx >= 0 && idx < statuses.length) {
        return statuses[idx];
      }
    }
    return 'unverified';
  }

  @override
  String toString() =>
      'ProvenanceEnvelope(id: $id, record: $collection:$recordId, confidence: $confidenceFactor)';
}
