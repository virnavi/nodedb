/// Cache expiry mode for a record.
enum CacheMode {
  /// TTL is measured from the document's `updatedAt` timestamp.
  expireAfterWrite,

  /// TTL is measured from the document's `createdAt` timestamp.
  expireAfterCreate,
}

/// Per-record cache configuration.
///
/// When set on a write operation, the record will automatically expire
/// after the specified [ttl] duration, measured from either the last
/// write time ([CacheMode.expireAfterWrite]) or creation time
/// ([CacheMode.expireAfterCreate]).
class CacheConfig {
  final CacheMode mode;
  final Duration ttl;

  const CacheConfig({
    this.mode = CacheMode.expireAfterWrite,
    required this.ttl,
  });

  /// Serialize to a map for MessagePack encoding.
  Map<String, dynamic> toMap() => {
        'mode': mode == CacheMode.expireAfterWrite
            ? 'expire_after_write'
            : 'expire_after_create',
        'ttl_secs': ttl.inSeconds,
      };

  /// Deserialize from a MessagePack map.
  static CacheConfig? fromMap(Map<String, dynamic>? map) {
    if (map == null) return null;
    final modeStr = map['mode'] as String?;
    final ttlSecs = map['ttl_secs'] as int?;
    if (modeStr == null || ttlSecs == null) return null;
    return CacheConfig(
      mode: modeStr == 'expire_after_create'
          ? CacheMode.expireAfterCreate
          : CacheMode.expireAfterWrite,
      ttl: Duration(seconds: ttlSecs),
    );
  }

  @override
  String toString() => 'CacheConfig(mode: ${mode.name}, ttl: ${ttl.inSeconds}s)';
}
