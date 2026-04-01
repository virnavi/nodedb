import '../util/msgpack.dart';

/// A data access control rule.
class AccessRule {
  final int id;
  final String collection;
  final String? field;
  final String? recordId;
  final String subjectType;
  final String subjectId;
  final String permission;
  final DateTime? expiresAt;

  const AccessRule({
    required this.id,
    required this.collection,
    this.field,
    this.recordId,
    required this.subjectType,
    required this.subjectId,
    required this.permission,
    this.expiresAt,
  });

  factory AccessRule.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final collection = (decodeField(decoded, 'collection', 1) ?? '') as String;
    final field = decodeField(decoded, 'field', 2) as String?;
    final recordId = decodeField(decoded, 'record_id', 3) as String?;
    final subjectType =
        _decodeEnum(decodeField(decoded, 'subject_type', 4), const ['peer', 'group'], 'peer');
    final subjectId = (decodeField(decoded, 'subject_id', 5) ?? '') as String;
    final permission =
        _decodeEnum(decodeField(decoded, 'permission', 6), const ['allow', 'deny', 'redact'], 'deny');
    final expiresAtStr = decodeField(decoded, 'expires_at', 7);
    final expiresAt =
        expiresAtStr is String ? DateTime.tryParse(expiresAtStr) : null;

    return AccessRule(
      id: id,
      collection: collection,
      field: field,
      recordId: recordId,
      subjectType: subjectType,
      subjectId: subjectId,
      permission: permission,
      expiresAt: expiresAt,
    );
  }

  /// Decode a Rust enum from either a string or positional [variant_idx, []].
  static String _decodeEnum(dynamic value, List<String> variants, String fallback) {
    if (value is String) return value;
    if (value is List && value.isNotEmpty) {
      final idx = value[0];
      if (idx is int && idx >= 0 && idx < variants.length) return variants[idx];
    }
    return fallback;
  }

  @override
  String toString() =>
      'AccessRule(id: $id, collection: $collection, $subjectType:$subjectId -> $permission)';
}
