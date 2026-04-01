import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';

import 'database_mesh.dart';
import 'engine/nosql_engine.dart';
import 'engine/graph_engine.dart';
import 'engine/vector_engine.dart';
import 'engine/federation_engine.dart';
import 'engine/dac_engine.dart';
import 'engine/transport_engine.dart';
import 'engine/provenance_engine.dart';
import 'engine/keyresolver_engine.dart';
import 'engine/ai_provenance_engine.dart';
import 'engine/ai_query_engine.dart';
import 'model/document.dart';
import 'model/keypair.dart';
import 'model/migration.dart';
import 'model/provenance_envelope.dart';
import 'model/ai_provenance.dart';
import 'model/ai_query.dart';
import 'model/trim.dart';
import 'adapter/ai_provenance_adapter.dart';
import 'adapter/ai_query_adapter.dart';
import 'query/query_result.dart';
import 'sync/collection_notifier.dart';

/// Serializable handle set for passing a [NodeDB] instance across isolates.
///
/// Obtain via [NodeDB.handles], then reconstruct via [NodeDB.attach] in the
/// target isolate. Handles are plain integers (u64) backed by Rust-side
/// `RwLock<HashMap>`, so they are valid across all isolates in the process.
class NodeDbHandles {
  final int nosql;
  final int federation;
  final int? graph;
  final int? vector;
  final int? dac;
  final int? transport;
  final int? provenance;
  final int? keyResolver;
  final int? aiProvenance;
  final int? aiQuery;

  const NodeDbHandles({
    required this.nosql,
    required this.federation,
    this.graph,
    this.vector,
    this.dac,
    this.transport,
    this.provenance,
    this.keyResolver,
    this.aiProvenance,
    this.aiQuery,
  });
}

/// Top-level facade for NodeDB — opens and manages all engines.
class NodeDB {
  final NoSqlEngine nosql;
  final GraphEngine? graph;
  final VectorEngine? vector;
  final FederationEngine federation;
  final DacEngine? dac;
  final TransportEngine? transport;
  final ProvenanceEngine? provenance;
  final KeyResolverEngine? keyResolver;
  final AiProvenanceEngine? aiProvenance;
  final AiQueryEngine? aiQuery;
  final DatabaseMesh? mesh;
  final String? databaseName;
  final String sharingStatus;
  final bool _ownsFederation;
  final NodeDbBindings _bindings;

  /// AI provenance adapter (user-provided). Set via [configureAiProvenance].
  NodeDbAiProvenanceAdapter? _aiProvenanceAdapter;

  /// AI query adapter (user-provided). Set via [configureAiQuery].
  NodeDbAiQueryAdapter? _aiQueryAdapter;

  /// AI provenance configuration.
  AiProvenanceConfig _aiProvenanceConfig = const AiProvenanceConfig();

  /// AI query configuration.
  AiQueryConfig _aiQueryConfig = const AiQueryConfig();

  /// Lazy collection notifier for reactive watch streams.
  CollectionNotifier? _notifier;

  /// Returns a [CollectionNotifier] that polls the sync version counter and
  /// broadcasts change events. Starts polling on first access.
  CollectionNotifier get notifier {
    _notifier ??= CollectionNotifier(nosql)..startPolling();
    return _notifier!;
  }

  NodeDB._({
    required this.nosql,
    required this.federation,
    required NodeDbBindings bindings,
    required bool ownsFederation,
    this.mesh,
    this.databaseName,
    this.sharingStatus = 'full',
    this.graph,
    this.vector,
    this.dac,
    this.transport,
    this.provenance,
    this.keyResolver,
    this.aiProvenance,
    this.aiQuery,
  })  : _ownsFederation = ownsFederation,
        _bindings = bindings;

  /// Serializable handle set for passing this instance across isolates.
  ///
  /// Send [handles] to another isolate, then call [NodeDB.attach] there
  /// to reconstruct a fully functional [NodeDB] without re-opening.
  NodeDbHandles get handles => NodeDbHandles(
        nosql: nosql.handle,
        federation: federation.handle,
        graph: graph?.handle,
        vector: vector?.handle,
        dac: dac?.handle,
        transport: transport?.handle,
        provenance: provenance?.handle,
        keyResolver: keyResolver?.handle,
        aiProvenance: aiProvenance?.handle,
        aiQuery: aiQuery?.handle,
      );

  /// Attach to an already-opened NodeDB from another isolate.
  ///
  /// Creates fresh [NodeDbBindings] (loads the native library) and wraps
  /// each handle. The resulting instance shares the same Rust-side engines
  /// and is fully thread-safe.
  ///
  /// **Important**: Do NOT call [close] on the attached instance — only
  /// close from the original owning isolate.
  static NodeDB attach(NodeDbHandles h, {NodeDbBindings? bindings}) {
    final b = bindings ?? NodeDbBindings(loadNodeDbLibrary());
    return NodeDB._(
      nosql: NoSqlEngine.fromHandle(b, h.nosql),
      federation: FederationEngine.fromHandle(b, h.federation),
      bindings: b,
      ownsFederation: false, // attached instances never own federation
      graph: h.graph != null ? GraphEngine.fromHandle(b, h.graph!) : null,
      vector: h.vector != null ? VectorEngine.fromHandle(b, h.vector!) : null,
      dac: h.dac != null ? DacEngine.fromHandle(b, h.dac!) : null,
      transport: h.transport != null
          ? TransportEngine.fromHandle(b, h.transport!)
          : null,
      provenance: h.provenance != null
          ? ProvenanceEngine.fromHandle(b, h.provenance!)
          : null,
      keyResolver: h.keyResolver != null
          ? KeyResolverEngine.fromHandle(b, h.keyResolver!)
          : null,
      aiProvenance: h.aiProvenance != null
          ? AiProvenanceEngine.fromHandle(b, h.aiProvenance!)
          : null,
      aiQuery: h.aiQuery != null
          ? AiQueryEngine.fromHandle(b, h.aiQuery!)
          : null,
    );
  }

  static void _linkTransport(NodeDbBindings b, int dbHandle, int transportHandle) {
    final outError = calloc<NodeDbErrorStruct>();
    try {
      final ok = b.linkTransport(dbHandle, transportHandle, outError);
      if (!ok) {
        final code = outError.ref.code;
        final msgPtr = outError.ref.message;
        final msg = msgPtr == nullptr ? 'link failed' : msgPtr.cast<Utf8>().toDartString();
        if (msgPtr != nullptr) b.freeError(outError);
        throw StateError('linkTransport error $code: $msg');
      }
    } finally {
      calloc.free(outError);
    }
  }

  /// Open a NodeDB instance with the given configuration.
  ///
  /// Provide a [DatabaseMesh] via [mesh] to enable transport networking
  /// and shared federation. Without a mesh, the database operates in
  /// local-only mode with its own federation engine.
  ///
  /// Only NoSQL is required. All other engines are opt-in via their
  /// respective config parameters.
  static NodeDB open({
    required String directory,
    required String databaseName,
    String sharingStatus = 'full',
    DatabaseMesh? mesh,
    bool graphEnabled = false,
    VectorOpenConfig? vectorConfig,
    bool dacEnabled = false,
    bool provenanceEnabled = false,
    bool keyResolverEnabled = false,
    NodeDbBindings? bindings,
  }) {
    final b = bindings ?? mesh?.bindings ?? NodeDbBindings(loadNodeDbLibrary());

    String subdir(String name) {
      final dir = Directory('$directory/$name');
      if (!dir.existsSync()) dir.createSync(recursive: true);
      return dir.path;
    }

    // NoSQL (always opened — uses base directory for backwards compatibility)
    final nosql = NoSqlEngine.open(
      b,
      directory,
      ownerPrivateKeyHex: mesh?.ownerPrivateKeyHex,
    );

    // Graph
    GraphEngine? graph;
    if (graphEnabled) {
      graph = GraphEngine.open(b, subdir('graph'));
    }

    // Vector
    VectorEngine? vector;
    if (vectorConfig != null) {
      vector = VectorEngine.open(b, vectorConfig);
    }

    // Federation — from mesh if provided, otherwise local
    FederationEngine federation;
    bool ownsFederation;
    if (mesh != null) {
      federation = mesh.federation;
      ownsFederation = false;
    } else {
      federation = FederationEngine.open(b, subdir('__mgmt__'));
      ownsFederation = true;
    }

    // DAC
    DacEngine? dac;
    if (dacEnabled) {
      dac = DacEngine.open(b, subdir('dac'));
    }

    // Transport — only via mesh
    TransportEngine? transport;
    if (mesh != null) {
      final allocatedAddr = mesh.allocateListenAddr();
      final fullConfig = <String, dynamic>{
        ...mesh.transportConfig.toMap(),
        'listen_addr': allocatedAddr,
        'mesh_name': mesh.meshName,
        'mesh_database_name': databaseName,
        'mesh_sharing_status': sharingStatus,
        if (mesh.meshSecret != null) 'mesh_secret': mesh.meshSecret,
        'nosql_handle': nosql.handle,
        'federation_handle': federation.handle,
      };
      transport = TransportEngine.open(b, fullConfig);
      _linkTransport(b, nosql.handle, transport.handle);
    }

    // Provenance
    ProvenanceEngine? provenance;
    if (provenanceEnabled) {
      provenance = ProvenanceEngine.open(b, subdir('provenance'));
    }

    // KeyResolver
    KeyResolverEngine? keyResolver;
    if (keyResolverEnabled) {
      keyResolver = KeyResolverEngine.open(b, subdir('keyresolver'));
    }

    // AI Provenance (requires provenance)
    AiProvenanceEngine? aiProvenance;
    if (provenance != null) {
      aiProvenance = AiProvenanceEngine.open(b, provenance.handle);
    }

    // AI Query (requires nosql + provenance)
    AiQueryEngine? aiQuery;
    if (provenance != null) {
      aiQuery = AiQueryEngine.open(
        b,
        nosqlHandle: nosql.handle,
        provenanceHandle: provenance.handle,
      );
    }

    return NodeDB._(
      nosql: nosql,
      federation: federation,
      bindings: b,
      ownsFederation: ownsFederation,
      mesh: mesh,
      databaseName: databaseName,
      sharingStatus: sharingStatus,
      graph: graph,
      vector: vector,
      dac: dac,
      transport: transport,
      provenance: provenance,
      keyResolver: keyResolver,
      aiProvenance: aiProvenance,
      aiQuery: aiQuery,
    );
  }

  /// Get the FFI library version.
  int get ffiVersion => _bindings.ffiVersion();

  /// Run a migration against the NoSQL database.
  MigrationResult runMigration(NodeMigration migration) =>
      nosql.runMigration(migration);

  /// Generate a new Ed25519 keypair via the native FFI.
  NodeDBKeyPair generateKeypair() => nosql.generateKeypair();

  /// Get the typed owner key status.
  OwnerKeyStatus get ownerKeyStatusTyped => nosql.ownerKeyStatusTyped;

  /// Rotate the database owner key.
  RotateKeyResult rotateOwnerKey(
    String currentPrivateKeyHex,
    String newPrivateKeyHex,
  ) =>
      nosql.rotateOwnerKey(currentPrivateKeyHex, newPrivateKeyHex);

  // ── Singleton delegates ─────────────────────────────────────────

  /// Create a singleton collection with default values.
  Document singletonCreate(String collection, Map<String, dynamic> defaults) =>
      nosql.singletonCreate(collection, defaults);

  /// Get the singleton document from a collection.
  Document singletonGet(String collection) => nosql.singletonGet(collection);

  /// Update the singleton document in a collection.
  Document singletonPut(String collection, Map<String, dynamic> data) =>
      nosql.singletonPut(collection, data);

  /// Reset a singleton to its declared defaults.
  Document singletonReset(String collection) => nosql.singletonReset(collection);

  /// Check if a collection is a singleton.
  bool isSingleton(String collection) => nosql.isSingleton(collection);

  // ── Collection delegates ─────────────────────────────────────

  /// Write a batch of operations atomically.
  void writeTxn(List<WriteOp> ops) => nosql.writeTxn(ops);

  /// Get a document by ID from a collection.
  Document? get(String collection, int id) => nosql.get(collection, id);

  /// Find all documents in a collection with optional filter/sort/pagination.
  List<Document> findAll(
    String collection, {
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
  }) =>
      nosql.findAll(collection,
          filter: filter, sort: sort, offset: offset, limit: limit);

  /// Count documents in a collection.
  int count(String collection) => nosql.count(collection);

  /// List all collection names (schema-qualified).
  List<String> collectionNames() => nosql.collectionNames();

  // ── Trigger delegates ───────────────────────────────────────

  /// Register a trigger on a collection.
  int registerTrigger({
    required String collection,
    required String event,
    required String timing,
    String? name,
  }) =>
      nosql.registerTrigger(
          collection: collection, event: event, timing: timing, name: name);

  /// Unregister a trigger by ID.
  bool unregisterTrigger(int triggerId) => nosql.unregisterTrigger(triggerId);

  /// Enable or disable a trigger.
  bool setTriggerEnabled(int triggerId, {bool enabled = true}) =>
      nosql.setTriggerEnabled(triggerId, enabled: enabled);

  // ── Preference delegates ──────────────────────────────────────

  /// Set a preference value.
  Map<String, dynamic> prefSet(
    String store,
    String key,
    dynamic value, {
    bool shareable = false,
    String conflictResolution = 'last_write_wins',
  }) =>
      nosql.prefSet(store, key, value,
          shareable: shareable, conflictResolution: conflictResolution);

  /// Get a preference value.
  dynamic prefGet(String store, String key) => nosql.prefGet(store, key);

  /// Remove a preference.
  bool prefRemove(String store, String key) => nosql.prefRemove(store, key);

  /// List all preference keys in a store.
  List<String> prefKeys(String store) => nosql.prefKeys(store);

  /// Get all shareable preference entries.
  List<Map<String, dynamic>> prefShareable(String store) =>
      nosql.prefShareable(store);

  // ── Access History delegates ─────────────────────────────────

  /// Query access history entries.
  List<Map<String, dynamic>> accessHistoryQuery({
    String? collection,
    int? recordId,
    String? eventType,
  }) =>
      nosql.accessHistoryQuery(
          collection: collection, recordId: recordId, eventType: eventType);

  /// Count total access history entries.
  int accessHistoryCount() => nosql.accessHistoryCount();

  /// Get the last access time for a collection/record.
  String? accessHistoryLastAccess(String collection, int recordId) =>
      nosql.accessHistoryLastAccess(collection, recordId);

  /// Trim old access history entries.
  int accessHistoryTrim({int retentionSecs = 365 * 24 * 3600}) =>
      nosql.accessHistoryTrim(retentionSecs: retentionSecs);

  // ── Trim delegates ──────────────────────────────────────────

  /// Get a trim recommendation.
  TrimRecommendation recommendTrim(
    TrimPolicy policy, {
    List<String> excludeCollections = const [],
  }) =>
      nosql.recommendTrim(policy, excludeCollections: excludeCollections);

  /// Trim a single collection.
  TrimReport trim(String collection, TrimPolicy policy,
          {bool dryRun = false}) =>
      nosql.trim(collection, policy, dryRun: dryRun);

  /// Trim all trimmable collections.
  TrimReport trimAll(TrimPolicy policy, {bool dryRun = false}) =>
      nosql.trimAll(policy, dryRun: dryRun);

  /// Execute a user-approved trim.
  TrimReport trimApproved(UserApprovedTrim approval) =>
      nosql.trimApproved(approval);

  /// Get the effective trim policy for a collection.
  TrimPolicy? trimConfigEffective(String collection) =>
      nosql.trimConfigEffective(collection);

  /// Check if a collection is never-trim.
  bool trimConfigIsNeverTrim(String collection) =>
      nosql.trimConfigIsNeverTrim(collection);

  /// Set a trim policy for a collection.
  void trimConfigSet(String collection, TrimPolicy policy) =>
      nosql.trimConfigSet(collection, policy);

  /// Reset a collection's trim config to default.
  void trimConfigReset(String collection) => nosql.trimConfigReset(collection);

  /// Mark a record as never-trim.
  void trimConfigSetRecordNeverTrim(String collection, int recordId) =>
      nosql.trimConfigSetRecordNeverTrim(collection, recordId);

  /// Clear a record-level trim override.
  void trimConfigClearRecordOverride(String collection, int recordId) =>
      nosql.trimConfigClearRecordOverride(collection, recordId);

  // ── Enhanced query methods ─────────────────────────────────

  /// Find documents with provenance envelopes attached.
  ///
  /// Each result includes the latest [ProvenanceEnvelope] for that record.
  /// If provenance engine is not enabled, envelopes will be null.
  List<WithProvenance<Document>> findAllWithProvenance(
    String collection, {
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
  }) {
    final docs = nosql.findAll(collection,
        filter: filter, sort: sort, offset: offset, limit: limit);
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (provenance != null) {
        final envelopes = provenance!.getForRecord(collection, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(doc, envelope);
    }).toList();
  }

  /// Find documents across federated peers via transport mesh query.
  ///
  /// Returns results tagged with the source peer ID. Local results use
  /// `"local"` as the peer ID. Requires transport engine to be enabled.
  List<FederatedResult<Document>> findAllFederated(
    String collection, {
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
    int timeoutSecs = 10,
    int ttl = 3,
  }) {
    // Start with local results
    final localDocs = nosql.findAll(collection,
        filter: filter, sort: sort, offset: offset, limit: limit);
    final results = localDocs
        .map((doc) => FederatedResult(doc, 'local'))
        .toList();

    // Query federated peers if transport is available
    if (transport != null) {
      try {
        final resp = transport!.federatedQuery(
          queryType: 'nosql',
          queryData: {
            'collection': collection,
            if (filter != null) 'filter': filter,
            if (sort != null) 'sort': sort,
            if (offset != null) 'offset': offset,
            if (limit != null) 'limit': limit,
          },
          timeoutSecs: timeoutSecs,
          ttl: ttl,
        );
        if (resp is List) {
          for (final item in resp) {
            if (item is Map) {
              final peerId = item['peer_id']?.toString() ?? 'unknown';
              final data = item['data'];
              if (data is List) {
                for (final d in data) {
                  results.add(FederatedResult(
                    Document.fromMsgpack(d),
                    peerId,
                  ));
                }
              } else if (data is Map) {
                results.add(FederatedResult(
                  Document.fromMsgpack(data),
                  peerId,
                ));
              }
            }
          }
        }
      } on Exception {
        // Federation failures are non-fatal — return local results only
      }
    }
    return results;
  }

  /// Find documents using AI query fallback when local results are empty.
  ///
  /// Requires [configureAiQuery] to have been called. If local results
  /// exist, returns them directly. Otherwise calls the AI adapter.
  Future<List<Document>> findAllWithAi(
    String collection, {
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
    String? queryDescription,
  }) async {
    // Try local first
    final localDocs = nosql.findAll(collection,
        filter: filter, sort: sort, offset: offset, limit: limit);
    if (localDocs.isNotEmpty) return localDocs;

    // Fall back to AI adapter
    if (_aiQueryAdapter == null || aiQuery == null) return [];

    final schema = nosql.schemaInfo(collection);
    final results = await _aiQueryAdapter!.queryForMissingData(
      collection: collection,
      schemaJson: schema?.toString() ?? '{}',
      queryDescription: queryDescription ?? 'Find documents in $collection',
      context: AiQueryContext(
        aiSubjectId: 'nodedb-query',
        maxResults: _aiQueryConfig.maxResultsPerQuery,
        minimumWriteConfidence: _aiQueryConfig.minimumWriteConfidence,
      ),
    );

    // Filter by minimum confidence and convert to FFI format
    final eligible = results
        .where((r) => r.confidence >= _aiQueryConfig.minimumWriteConfidence)
        .map((r) => r.toFfiMap())
        .toList();
    if (eligible.isEmpty) return [];

    // Process results through AI query engine for provenance tracking
    try {
      final resp = aiQuery!.processResults(
        collection: collection,
        results: eligible,
      );
      if (resp is List) {
        return resp.map((item) => Document.fromMsgpack(item)).toList();
      }
    } on Exception {
      // AI processing failures are non-fatal
    }
    return [];
  }

  /// Full query pipeline: local → federation → AI fallback.
  ///
  /// Returns results with provenance envelopes attached. Tries local
  /// first, then federated peers, then AI adapter as last resort.
  Future<List<WithProvenance<Document>>> findAllFull(
    String collection, {
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
    String? queryDescription,
    int federationTimeoutSecs = 10,
  }) async {
    // Try local
    var docs = nosql.findAll(collection,
        filter: filter, sort: sort, offset: offset, limit: limit);

    // Try federation if local is empty
    if (docs.isEmpty && transport != null) {
      final fedResults = findAllFederated(collection,
          filter: filter, sort: sort, offset: offset, limit: limit,
          timeoutSecs: federationTimeoutSecs);
      docs = fedResults.map((r) => r.data).toList();
    }

    // Try AI if still empty
    if (docs.isEmpty && _aiQueryAdapter != null) {
      docs = await findAllWithAi(collection,
          filter: filter, sort: sort, offset: offset, limit: limit,
          queryDescription: queryDescription);
    }

    // Attach provenance to all results
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (provenance != null) {
        final envelopes = provenance!.getForRecord(collection, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(doc, envelope);
    }).toList();
  }

  // ── Cross-engine queries ────────────────────────────────────

  /// Find documents with DAC filtering applied.
  ///
  /// Returns only documents/fields that [peerId] is allowed to see
  /// according to DAC rules for [collection].
  List<Document> findAllFiltered(
    String collection, {
    required String peerId,
    List<String>? groupIds,
    Map<String, dynamic>? filter,
    List<Map<String, dynamic>>? sort,
    int? offset,
    int? limit,
  }) {
    if (dac == null) {
      return nosql.findAll(collection,
          filter: filter, sort: sort, offset: offset, limit: limit);
    }
    final docs = nosql.findAll(collection,
        filter: filter, sort: sort, offset: offset, limit: limit);
    final filtered = <Document>[];
    for (final doc in docs) {
      final result = dac!.filterDocument(
        collection: collection,
        document: doc.data,
        peerId: peerId,
        groupIds: groupIds,
        recordId: doc.id.toString(),
      );
      if (result != null) {
        filtered.add(Document(
          id: doc.id,
          collection: doc.collection,
          data: result,
          createdAt: doc.createdAt,
          updatedAt: doc.updatedAt,
        ));
      }
    }
    return filtered;
  }

  /// Get a document with its provenance envelope.
  WithProvenance<Document>? getWithProvenance(String collection, int id) {
    final doc = nosql.get(collection, id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (provenance != null) {
      final envelopes = provenance!.getForRecord(collection, id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(doc, envelope);
  }

  /// Get a graph node with all its incoming and outgoing edges.
  Map<String, dynamic>? getNodeWithEdges(int nodeId) {
    if (graph == null) throw StateError('Graph engine is not open');
    final node = graph!.getNode(nodeId);
    if (node == null) return null;
    return {
      'node': node,
      'edgesFrom': graph!.edgesFrom(nodeId),
      'edgesTo': graph!.edgesTo(nodeId),
    };
  }

  // ── Schema delegates ──────────────────────────────────────────

  /// Create a new schema.
  void createSchema(String name, {String? sharingStatus}) =>
      nosql.createSchema(name, sharingStatus: sharingStatus);

  /// List all schemas.
  List<Map<String, dynamic>> listSchemas() => nosql.listSchemas();

  /// Get the schema fingerprint.
  String schemaFingerprint() => nosql.schemaFingerprint();

  // ── AI Adapter Configuration ──────────────────────────────────

  /// Configure the AI provenance adapter.
  ///
  /// Requires provenance engine to be enabled. The adapter will be called
  /// to assess records, resolve conflicts, detect anomalies, and classify
  /// data sources.
  void configureAiProvenance({
    required NodeDbAiProvenanceAdapter adapter,
    AiProvenanceConfig config = const AiProvenanceConfig(),
  }) {
    if (aiProvenance == null) {
      throw StateError(
          'AI provenance requires provenanceEnabled: true');
    }
    _aiProvenanceAdapter = adapter;
    _aiProvenanceConfig = config;
  }

  /// Configure the AI query fallback adapter.
  ///
  /// Requires provenance engine to be enabled. The adapter will be called
  /// when local and federated queries return no results.
  void configureAiQuery({
    required NodeDbAiQueryAdapter adapter,
    AiQueryConfig config = const AiQueryConfig(),
  }) {
    if (aiQuery == null) {
      throw StateError(
          'AI query requires provenanceEnabled: true');
    }
    _aiQueryAdapter = adapter;
    _aiQueryConfig = config;
  }

  /// The current AI provenance adapter, or null if not configured.
  NodeDbAiProvenanceAdapter? get aiProvenanceAdapter => _aiProvenanceAdapter;

  /// The current AI query adapter, or null if not configured.
  NodeDbAiQueryAdapter? get aiQueryAdapter => _aiQueryAdapter;

  /// The current AI provenance configuration.
  AiProvenanceConfig get aiProvenanceConfig => _aiProvenanceConfig;

  /// The current AI query configuration.
  AiQueryConfig get aiQueryConfig => _aiQueryConfig;

  /// Close all open engines and release resources.
  ///
  /// If this database was opened with a [DatabaseMesh], the shared
  /// federation engine is NOT closed — close the mesh separately.
  void close() {
    _notifier?.dispose();
    _notifier = null;
    aiQuery?.close();
    aiProvenance?.close();
    keyResolver?.close();
    provenance?.close();
    transport?.close();
    dac?.close();
    if (_ownsFederation) federation.close();
    vector?.close();
    graph?.close();
    nosql.close();
  }
}
