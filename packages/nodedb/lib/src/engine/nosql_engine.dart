import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/cache_config.dart';
import '../model/document.dart';
import '../model/keypair.dart';
import '../model/migration.dart';
import '../model/trim.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB NoSQL engine.
class NoSqlEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  NoSqlEngine._(this._handle, this._bindings);

  /// Attach to an existing NoSQL engine handle (for multi-isolate use).
  ///
  /// The handle must have been opened in another isolate. The caller must
  /// NOT close this instance — close only from the owning isolate.
  factory NoSqlEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return NoSqlEngine._(handle, bindings);
  }

  /// Open a NoSQL database at [path].
  static NoSqlEngine open(
    NodeDbBindings bindings,
    String path, {
    String? ownerPrivateKeyHex,
  }) {
    final config = buildConfig(path, {
      if (ownerPrivateKeyHex != null)
        'owner_private_key_hex': ownerPrivateKeyHex,
    });
    final handle = openRaw(bindings, bindings.open, config);
    return NoSqlEngine._(handle, bindings);
  }

  int get handle => _handle;

  // ── Query actions ───────────────────────────────────────────────

  /// Get a document by ID from a collection.
  /// Returns null if the document does not exist.
  Document? get(String collection, int id) {
    try {
      final resp =
          _query({'action': 'get', 'collection': collection, 'id': id});
      if (resp == null) return null;
      return Document.fromMsgpack(resp);
    } on NotFoundException {
      return null;
    }
  }

  /// Find all documents in a collection with optional filter, sort, and pagination.
  ///
  /// When [filter] or [sort] are provided, uses the Rust `query` action which
  /// supports full Query processing (filter, sort, offset, limit).
  /// Otherwise uses the simpler `find_all` action with offset/limit only.
  List<Document> findAll(
    String collection, {
    int? offset,
    int? limit,
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
  }) {
    if (filter != null || sort != null) {
      // Use the "query" action which supports full Query struct
      final query = <String, dynamic>{};
      if (filter != null) query['filter'] = filter;
      if (sort != null) query['sort'] = sort;
      if (offset != null) query['offset'] = offset;
      if (limit != null) query['limit'] = limit;

      final resp = _query({
        'action': 'query',
        'collection': collection,
        'query': query,
      });
      if (resp == null || resp is! List) return [];
      return resp.map((item) => Document.fromMsgpack(item)).toList();
    }

    // Simple find_all with offset/limit only
    final fields = <String, dynamic>{
      'action': 'find_all',
      'collection': collection,
    };
    if (offset != null) fields['offset'] = offset;
    if (limit != null) fields['limit'] = limit;

    final resp = _query(fields);
    if (resp == null || resp is! List) return [];
    return resp.map((item) => Document.fromMsgpack(item)).toList();
  }

  /// Count documents in a collection.
  int count(String collection) {
    final resp = _query({'action': 'count', 'collection': collection});
    return (resp is int) ? resp : 0;
  }

  /// Clear all documents in a collection. Returns the count of deleted records.
  int clear(String collection) {
    final resp = _query({'action': 'clear', 'collection': collection});
    return (resp is int) ? resp : 0;
  }

  /// Fast bulk insert/update — bypasses triggers, notifications, and access history.
  /// Each item must have a 'data' map and optionally an 'id' (0 = auto-generate).
  /// Returns the count of successfully written records.
  int batchPut(String collection, List<Map<String, dynamic>> items) {
    final resp = _query({
      'action': 'batch_put',
      'collection': collection,
      'items': items,
    });
    return (resp is int) ? resp : 0;
  }

  /// Fast bulk delete by IDs — bypasses triggers, notifications, and access history.
  /// Returns the count of successfully deleted records.
  int batchDelete(String collection, List<int> ids) {
    final resp = _query({
      'action': 'batch_delete',
      'collection': collection,
      'ids': ids,
    });
    return (resp is int) ? resp : 0;
  }

  // ── Write transaction ───────────────────────────────────────────

  /// Execute a write transaction with a list of operations.
  void writeTxn(List<WriteOp> operations) {
    final ops = operations.map((op) => op.toMap()).toList();
    final bytes = msgpackEncode(ops);
    try {
      writeTxnRaw(_bindings, _bindings.writeTxn, _handle, bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }

  // ── Schema actions ──────────────────────────────────────────────

  void createSchema(String name, {String? sharingStatus}) {
    _query({
      'action': 'create_schema',
      'name': name,
      if (sharingStatus != null) 'sharing_status': sharingStatus,
    });
  }

  void dropSchema(String name) {
    _query({'action': 'drop_schema', 'name': name});
  }

  List<Map<String, dynamic>> listSchemas() {
    final resp = _query({'action': 'list_schemas'});
    if (resp is! List) return [];
    return resp
        .map((s) => s is Map ? Map<String, dynamic>.from(s) : <String, dynamic>{})
        .toList();
  }

  Map<String, dynamic>? schemaInfo(String name) {
    final resp = _query({'action': 'schema_info', 'name': name});
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return null;
  }

  List<String> collectionNames() {
    final resp = _query({'action': 'collection_names'});
    if (resp is! List) return [];
    return resp.map((s) => s.toString()).toList();
  }

  String schemaFingerprint() {
    final resp = _query({'action': 'schema_fingerprint'});
    return resp?.toString() ?? '';
  }

  /// Move a collection to a different schema.
  ///
  /// [from] is the schema-qualified collection name (e.g., "public.items").
  /// [toSchema] is the target schema name (e.g., "other").
  void moveCollection(String from, String toSchema) {
    _query({
      'action': 'move_collection',
      'from': from,
      'to_schema': toSchema,
    });
  }

  /// Rename a schema.
  void renameSchema(String from, String to) {
    _query({
      'action': 'rename_schema',
      'from': from,
      'to': to,
    });
  }

  /// List collection names within a specific schema.
  List<String> collectionNamesInSchema(String schema) {
    final resp = _query({
      'action': 'collection_names_in_schema',
      'schema': schema,
    });
    if (resp is! List) return [];
    return resp.map((s) => s.toString()).toList();
  }

  // ── Singleton actions ───────────────────────────────────────────

  Document singletonCreate(
    String collection,
    Map<String, dynamic> defaults,
  ) {
    _query({
      'action': 'singleton_create',
      'collection': collection,
      'defaults': defaults,
    });
    return singletonGet(collection);
  }

  Document singletonGet(String collection) {
    final resp = _query({
      'action': 'singleton_get',
      'collection': collection,
    });
    return Document.fromMsgpack(resp);
  }

  Document singletonPut(
    String collection,
    Map<String, dynamic> data,
  ) {
    final resp = _query({
      'action': 'singleton_put',
      'collection': collection,
      'data': data,
    });
    return Document.fromMsgpack(resp);
  }

  Document singletonReset(String collection) {
    final resp = _query({
      'action': 'singleton_reset',
      'collection': collection,
    });
    return Document.fromMsgpack(resp);
  }

  bool isSingleton(String collection) {
    final resp = _query({
      'action': 'is_singleton',
      'collection': collection,
    });
    if (resp is Map) return resp['is_singleton'] == true;
    return resp == true;
  }

  // ── Preference actions ──────────────────────────────────────────

  Map<String, dynamic> prefSet(
    String store,
    String key,
    dynamic value, {
    bool shareable = false,
    String conflictResolution = 'last_write_wins',
  }) {
    final resp = _query({
      'action': 'pref_set',
      'store': store,
      'key': key,
      'value': value,
      'shareable': shareable,
      'conflict_resolution': conflictResolution,
    });
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  dynamic prefGet(String store, String key) {
    final resp = _query({
      'action': 'pref_get',
      'store': store,
      'key': key,
    });
    return resp;
  }

  bool prefRemove(String store, String key) {
    final resp = _query({
      'action': 'pref_remove',
      'store': store,
      'key': key,
    });
    if (resp is Map) return resp['removed'] == true;
    return false;
  }

  List<String> prefKeys(String store) {
    final resp = _query({'action': 'pref_keys', 'store': store});
    if (resp is Map) {
      final keys = resp['keys'];
      if (keys is List) return keys.map((s) => s.toString()).toList();
    }
    if (resp is List) return resp.map((s) => s.toString()).toList();
    return [];
  }

  List<Map<String, dynamic>> prefShareable(String store) {
    final resp = _query({'action': 'pref_shareable', 'store': store});
    List? entries;
    if (resp is Map) {
      entries = resp['entries'] as List?;
    } else if (resp is List) {
      entries = resp;
    }
    if (entries == null) return [];
    return entries
        .map((s) => s is Map ? Map<String, dynamic>.from(s) : <String, dynamic>{})
        .toList();
  }

  // ── Trigger actions ─────────────────────────────────────────────

  int registerTrigger({
    required String collection,
    required String event,
    required String timing,
    String? name,
  }) {
    final resp = _query({
      'action': 'register_trigger',
      'collection': collection,
      'event': event,
      'timing': timing,
      if (name != null) 'name': name,
    });
    if (resp is Map) return (resp['trigger_id'] as num?)?.toInt() ?? 0;
    return (resp is int) ? resp : 0;
  }

  int registerMeshTrigger({
    required String sourceDatabase,
    required String collection,
    required String event,
    String timing = 'after',
    String? name,
  }) {
    final resp = _query({
      'action': 'register_mesh_trigger',
      'source_database': sourceDatabase,
      'collection': collection,
      'event': event,
      'timing': timing,
      if (name != null) 'name': name,
    });
    if (resp is Map) return (resp['trigger_id'] as num?)?.toInt() ?? 0;
    return (resp is int) ? resp : 0;
  }

  bool unregisterTrigger(int triggerId) {
    final resp =
        _query({'action': 'unregister_trigger', 'trigger_id': triggerId});
    if (resp is Map) return resp['removed'] == true;
    return false;
  }

  bool setTriggerEnabled(int triggerId, {bool enabled = true}) {
    final resp = _query({
      'action': 'set_trigger_enabled',
      'trigger_id': triggerId,
      'enabled': enabled,
    });
    if (resp is Map) return resp['found'] == true;
    return false;
  }

  List<Map<String, dynamic>> listTriggers() {
    final resp = _query({'action': 'list_triggers'});
    if (resp is! List) return [];
    return resp
        .map((s) => s is Map ? Map<String, dynamic>.from(s) : <String, dynamic>{})
        .toList();
  }

  // ── Access History actions ──────────────────────────────────

  /// Query access history entries.
  ///
  /// Filter by [collection], [recordId], or [eventType].
  List<Map<String, dynamic>> accessHistoryQuery({
    String? collection,
    int? recordId,
    String? eventType,
  }) {
    final resp = _query({
      'action': 'access_history_query',
      if (collection != null) 'collection': collection,
      if (recordId != null) 'record_id': recordId,
      if (eventType != null) 'event_type': eventType,
    });
    if (resp is! List) return [];
    return resp
        .map((item) => item is Map ? Map<String, dynamic>.from(item) : <String, dynamic>{})
        .toList();
  }

  /// Count total access history entries.
  int accessHistoryCount() {
    final resp = _query({'action': 'access_history_count'});
    return (resp is int) ? resp : 0;
  }

  /// Get the last access time for a collection/record.
  String? accessHistoryLastAccess(String collection, int recordId) {
    final resp = _query({
      'action': 'access_history_last_access',
      'collection': collection,
      'record_id': recordId,
    });
    if (resp == null || resp is! String) return null;
    return resp;
  }

  /// Trim old access history entries beyond [retentionSecs].
  int accessHistoryTrim({int retentionSecs = 365 * 24 * 3600}) {
    final resp = _query({
      'action': 'access_history_trim',
      'retention_secs': retentionSecs,
    });
    return (resp is int) ? resp : 0;
  }

  // ── Trim actions ──────────────────────────────────────────────

  /// Get a trim recommendation for the given [policy].
  TrimRecommendation recommendTrim(
    TrimPolicy policy, {
    List<String> excludeCollections = const [],
  }) {
    final resp = _query({
      'action': 'recommend_trim',
      'policy': policy.toMap(),
      if (excludeCollections.isNotEmpty)
        'exclude_collections': excludeCollections,
    });
    if (resp is Map) {
      return TrimRecommendation.fromMap(Map<String, dynamic>.from(resp));
    }
    return TrimRecommendation(
      totalCandidateCount: 0,
      byCollection: [],
      generatedAtUtc: '',
    );
  }

  /// Trim a single collection using [policy].
  TrimReport trim(
    String collection,
    TrimPolicy policy, {
    bool dryRun = false,
  }) {
    final resp = _query({
      'action': 'trim',
      'collection': collection,
      'policy': policy.toMap(),
      'dry_run': dryRun,
    });
    if (resp is Map) {
      return TrimReport.fromMap(Map<String, dynamic>.from(resp));
    }
    return TrimReport(
      collection: collection,
      candidateCount: 0,
      deletedCount: 0,
      skippedCount: 0,
      neverTrimSkippedCount: 0,
      triggerAbortedCount: 0,
      dryRun: dryRun,
      executedAtUtc: '',
      deletedRecordIds: [],
    );
  }

  /// Trim all trimmable collections using [policy].
  TrimReport trimAll(TrimPolicy policy, {bool dryRun = false}) {
    final resp = _query({
      'action': 'trim_all',
      'policy': policy.toMap(),
      'dry_run': dryRun,
    });
    if (resp is Map) {
      return TrimReport.fromMap(Map<String, dynamic>.from(resp));
    }
    return TrimReport(
      collection: '',
      candidateCount: 0,
      deletedCount: 0,
      skippedCount: 0,
      neverTrimSkippedCount: 0,
      triggerAbortedCount: 0,
      dryRun: dryRun,
      executedAtUtc: '',
      deletedRecordIds: [],
    );
  }

  /// Execute a user-approved trim.
  TrimReport trimApproved(UserApprovedTrim approval) {
    final req = <String, dynamic>{
      'action': 'trim_approved',
      ...approval.toMap(),
    };
    final resp = _query(req);
    if (resp is Map) {
      return TrimReport.fromMap(Map<String, dynamic>.from(resp));
    }
    return TrimReport(
      collection: '',
      candidateCount: 0,
      deletedCount: 0,
      skippedCount: 0,
      neverTrimSkippedCount: 0,
      triggerAbortedCount: 0,
      dryRun: false,
      executedAtUtc: '',
      deletedRecordIds: [],
    );
  }

  // ── Trim Config actions ───────────────────────────────────────

  /// Get the effective trim policy for a collection (null = never-trim).
  TrimPolicy? trimConfigEffective(String collection) {
    final resp = _query({
      'action': 'trim_config_effective',
      'collection': collection,
    });
    if (resp == null) return null;
    if (resp is Map) {
      return TrimPolicy.fromMap(Map<String, dynamic>.from(resp));
    }
    return null;
  }

  /// Check if a collection is never-trim (default).
  bool trimConfigIsNeverTrim(String collection) {
    final resp = _query({
      'action': 'trim_config_is_never_trim',
      'collection': collection,
    });
    return resp == true;
  }

  /// Set a trim policy for a collection (makes it trimmable).
  void trimConfigSet(String collection, TrimPolicy policy) {
    _query({
      'action': 'trim_config_set',
      'collection': collection,
      'policy': policy.toMap(),
    });
  }

  /// Reset a collection's trim policy to annotation default (never-trim).
  void trimConfigReset(String collection) {
    _query({
      'action': 'trim_config_reset',
      'collection': collection,
    });
  }

  /// Mark a specific record as never-trim.
  void trimConfigSetRecordNeverTrim(String collection, int recordId) {
    _query({
      'action': 'trim_config_set_record_never_trim',
      'collection': collection,
      'record_id': recordId,
    });
  }

  /// Clear a record-level trim override.
  void trimConfigClearRecordOverride(String collection, int recordId) {
    _query({
      'action': 'trim_config_clear_record_override',
      'collection': collection,
      'record_id': recordId,
    });
  }

  // ── Record Cache actions ────────────────────────────────────────

  /// Set cache configuration for a specific record.
  void setRecordCache(String collection, int recordId, CacheConfig config) {
    _query({
      'action': 'set_record_cache',
      'collection': collection,
      'record_id': recordId,
      'cache': config.toMap(),
    });
  }

  /// Get cache configuration for a specific record.
  /// Returns null if no cache config is set.
  CacheConfig? getRecordCache(String collection, int recordId) {
    final resp = _query({
      'action': 'get_record_cache',
      'collection': collection,
      'record_id': recordId,
    });
    if (resp == null || resp is! Map) return null;
    return CacheConfig.fromMap(Map<String, dynamic>.from(resp));
  }

  /// Clear cache configuration for a specific record.
  void clearRecordCache(String collection, int recordId) {
    _query({
      'action': 'clear_record_cache',
      'collection': collection,
      'record_id': recordId,
    });
  }

  /// Sweep expired cached records in a specific collection.
  /// Returns the count of deleted records.
  int sweepExpired(String collection) {
    final resp = _query({
      'action': 'sweep_expired',
      'collection': collection,
    });
    return (resp is int) ? resp : 0;
  }

  /// Sweep expired cached records across all collections.
  /// Returns the total count of deleted records.
  int sweepAllExpired() {
    final resp = _query({'action': 'sweep_all_expired'});
    return (resp is int) ? resp : 0;
  }

  // ── DB-level actions ────────────────────────────────────────────

  Map<String, dynamic> ownerKeyStatus() {
    final bytes = executeRaw(
      _bindings,
      _bindings.dbExecute,
      _handle,
      buildRequest('owner_key_status'),
    );
    final resp = msgpackDecode(bytes);
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return {};
  }

  /// Get the typed owner key status for this database.
  OwnerKeyStatus get ownerKeyStatusTyped {
    final resp = ownerKeyStatus();
    return OwnerKeyStatus.fromString(resp['status']?.toString() ?? 'unbound');
  }

  /// Generate a new Ed25519 keypair via the Rust FFI.
  NodeDBKeyPair generateKeypair() {
    final bytes = executeRaw(
      _bindings,
      _bindings.dbExecute,
      _handle,
      buildRequest('generate_keypair'),
    );
    final resp = msgpackDecode(bytes);
    if (resp is! Map) {
      throw NodeDbException(6, 'unexpected generate_keypair response');
    }
    return NodeDBKeyPair(
      privateKeyHex: resp['private_key_hex'] as String,
      publicKeyHex: resp['public_key_hex'] as String,
    );
  }

  /// Rotate the database owner key.
  ///
  /// Requires the current private key to unseal the DEK, then re-seals
  /// under the new key.
  RotateKeyResult rotateOwnerKey(
    String currentPrivateKeyHex,
    String newPrivateKeyHex,
  ) {
    final bytes = executeRaw(
      _bindings,
      _bindings.dbExecute,
      _handle,
      buildRequest('rotate_owner_key', {
        'current_private_key_hex': currentPrivateKeyHex,
        'new_private_key_hex': newPrivateKeyHex,
      }),
    );
    final resp = msgpackDecode(bytes);
    if (resp is! Map) {
      throw NodeDbException(6, 'unexpected rotate_owner_key response');
    }
    return RotateKeyResult(
      status: resp['status']?.toString() ?? 'unknown',
      newFingerprint: resp['new_fingerprint']?.toString() ?? '',
    );
  }

  /// Sign UTF-8 data with an Ed25519 private key via FFI.
  ///
  /// Returns hex-encoded signature (128 chars).
  String signData(String privateKeyHex, String payloadUtf8) {
    final bytes = executeRaw(
      _bindings,
      _bindings.dbExecute,
      _handle,
      buildRequest('sign', {
        'private_key_hex': privateKeyHex,
        'payload_utf8': payloadUtf8,
      }),
    );
    final resp = msgpackDecode(bytes);
    if (resp is! Map) {
      throw NodeDbException(6, 'unexpected sign response');
    }
    return resp['signature_hex'] as String;
  }

  /// Run a migration against the database.
  ///
  /// If the database is already at or past [migration.toVersion],
  /// the migration is a no-op.
  MigrationResult runMigration(NodeMigration migration) {
    final ctx = MigrationContext();
    migration.migrate(ctx);

    final bytes = executeRaw(
      _bindings,
      _bindings.dbExecute,
      _handle,
      buildRequest('migrate', {
        'target_version': migration.toVersion,
        'operations': ctx.operations,
      }),
    );
    final resp = msgpackDecode(bytes);
    if (resp is Map) {
      return MigrationResult(
        status: resp['status']?.toString() ?? 'unknown',
        version: (resp['version'] as num?)?.toInt() ?? migration.toVersion,
      );
    }
    return MigrationResult(status: 'migrated', version: migration.toVersion);
  }

  /// Close the database and release resources.
  void close() {
    _bindings.close(_handle);
  }

  // ── Sync ────────────────────────────────────────────────────────

  /// Returns the current sync version counter (monotonically increasing on
  /// every write — local or remote-applied).
  int syncVersion() {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.dbExecute,
        _handle,
        buildRequest('sync_version'),
      );
      final resp = msgpackDecode(bytes);
      return (resp is int) ? resp : 0;
    } on NodeDbFfiException {
      return 0;
    }
  }

  // ── Internal ────────────────────────────────────────────────────

  dynamic _query(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.query,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
