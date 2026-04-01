import 'model_info.dart';

/// Generates a base DAO class for a `@collection` model.
///
/// The generated DAO provides typed CRUD, pagination, and filter methods
/// that delegate to [NoSqlEngine] via [CollectionAccessor].
String generateDao(ModelInfo model) {
  if (model.type == ModelType.edge) return _generateEdgeDao(model);
  if (model.type == ModelType.node) return _generateNodeDao(model);
  return _generateCollectionDao(model);
}

String _generateCollectionDao(ModelInfo model) {
  if (model.singleton) return _generateSingletonDao(model);

  final cls = model.className;
  final daoName = '${cls}DaoBase';

  var buf = '''
// ── DAO ────────────────────────────────────────────────────────
abstract class $daoName {
  NoSqlEngine get engine;
  ProvenanceEngine? get provenanceEngine => null;
  CollectionNotifier? get notifier => null;
  String? get databaseName => null;

  String get collectionName => '${model.collectionName}';
  static const schemaName = '${model.schema ?? 'public'}';
  String get qualifiedName {
    final db = databaseName;
    if (db != null && db.isNotEmpty) return '\$db.\$schemaName.\$collectionName';
    return '\$schemaName.\$collectionName';
  }

  $cls _fromDocument(Document doc) => _\$${cls}FromMap(doc.data);
  Map<String, dynamic> _toMap($cls item) => _\$${cls}ToMap(item);
''';

  if (model.isStringId) {
    buf += _generateStringIdMethods(cls);
  } else {
    buf += _generateIntIdMethods(cls);
  }

  // Shared methods (unchanged regardless of id type)
  buf += _generateSharedMethods(cls);

  // Trim methods
  if (model.trimmable) {
    buf += '''

  bool get isTrimmable => true;

  TrimReport trim(TrimPolicy policy, {bool dryRun = false}) {
    return engine.trim(collectionName, policy, dryRun: dryRun);
  }

  TrimRecommendation recommendTrim(TrimPolicy policy) {
    return engine.recommendTrim(policy);
  }

  void setTrimPolicy(TrimPolicy policy) {
    engine.trimConfigSet(collectionName, policy);
  }

  TrimPolicy? get effectiveTrimPolicy => engine.trimConfigEffective(collectionName);
''';
  }

  if (model.neverTrim) {
    buf += '''

  bool get isNeverTrim => true;
''';
  }

  // Trigger methods
  if (model.triggers.isNotEmpty) {
    buf += '''

  List<int> registerDeclaredTriggers() {
    final ids = <int>[];
''';
    for (final t in model.triggers) {
      buf += '''
    ids.add(engine.registerTrigger(
      collection: collectionName,
      event: '${t.event}',
      timing: '${t.timing}',
${t.name != null ? "      name: '${t.name}'," : ''}
    ));
''';
    }
    buf += '''
    return ids;
  }
''';
  }

  buf += '''
}
''';

  return buf;
}

String _generateStringIdMethods(String cls) {
  return '''

  Document? _findDocumentById(String id) {
    final docs = engine.findAll(
      collectionName,
      filter: {'Condition': {'EqualTo': {'field': 'id', 'value': id}}},
      limit: 1,
    );
    return docs.isEmpty ? null : docs.first;
  }

  $cls? findById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create($cls item) {
    final map = _toMap(item);
    if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    engine.writeTxn([WriteOp.put(collectionName, data: map)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache($cls item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    engine.writeTxn([WriteOp.put(collectionName, data: map, cache: cache)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<$cls> items) {
    engine.writeTxn(
      items.map((item) {
        final map = _toMap(item);
        if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
          map['id'] = generateNodeDbId();
        }
        return WriteOp.put(collectionName, data: map);
      }).toList(),
    );
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save($cls item) {
    final map = _toMap(item);
    if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id),
    ]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache($cls item, CacheConfig cache) {
    final map = _toMap(item);
    if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
      map['id'] = generateNodeDbId();
    }
    final existing = _findDocumentById(map['id'] as String);
    engine.writeTxn([
      WriteOp.put(collectionName, data: map, id: existing?.id, cache: cache),
    ]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<$cls> items) {
    final ops = <WriteOp>[];
    for (final item in items) {
      final map = _toMap(item);
      if (map['id'] == null || (map['id'] is String && (map['id'] as String).isEmpty)) {
        map['id'] = generateNodeDbId();
      }
      final existing = _findDocumentById(map['id'] as String);
      ops.add(WriteOp.put(collectionName, data: map, id: existing?.id));
    }
    engine.writeTxn(ops);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  $cls? updateById(String id, $cls Function($cls current) modifier) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    final current = _fromDocument(doc);
    final updated = modifier(current);
    final map = _toMap(updated);
    map['id'] = id;
    engine.writeTxn([WriteOp.put(collectionName, data: map, id: doc.id)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return false;
    engine.writeTxn([WriteOp.delete(collectionName, id: doc.id)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<String> ids) {
    final ops = <WriteOp>[];
    for (final id in ids) {
      final doc = _findDocumentById(id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      engine.writeTxn(ops);
      notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
  }

  int deleteWhere(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    final items = findWhere(filter);
    final ops = <WriteOp>[];
    for (final item in items) {
      final doc = _findDocumentById(item.id);
      if (doc != null) {
        ops.add(WriteOp.delete(collectionName, id: doc.id));
      }
    }
    if (ops.isNotEmpty) {
      engine.writeTxn(ops);
      notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    }
    return ops.length;
  }

  WithProvenance<$cls>? findByIdWithProvenance(String id) {
    final doc = _findDocumentById(id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (provenanceEngine != null) {
      final envelopes = provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<$cls>> watchAll({FilterQuery<$cls>? query, bool fireImmediately = true}) {
    if (notifier == null) return Stream.value(findAll(query));
    return notifier!.watch<List<$cls>>(collectionName, () => findAll(query), fireImmediately: fireImmediately);
  }

  Stream<$cls?> watchById(String id, {bool fireImmediately = true}) {
    if (notifier == null) return Stream.value(findById(id));
    return notifier!.watch<$cls?>(collectionName, () => findById(id), fireImmediately: fireImmediately);
  }

  Stream<List<$cls>> watchWhere(
    FilterQuery<$cls> Function(FilterQuery<$cls>) filter, {
    bool fireImmediately = true,
  }) => watchAll(query: filter(FilterQuery<$cls>()), fireImmediately: fireImmediately);
''';
}

String _generateIntIdMethods(String cls) {
  return '''

  $cls? findById(int id) {
    final doc = engine.get(collectionName, id);
    if (doc == null) return null;
    return _fromDocument(doc);
  }

  void create($cls item) {
    engine.writeTxn([WriteOp.put(collectionName, data: _toMap(item))]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createWithCache($cls item, CacheConfig cache) {
    engine.writeTxn([WriteOp.put(collectionName, data: _toMap(item), cache: cache)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void createAll(List<$cls> items) {
    engine.writeTxn(
      items.map((item) => WriteOp.put(collectionName, data: _toMap(item))).toList(),
    );
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void save($cls item, {int? id}) {
    engine.writeTxn([WriteOp.put(collectionName, data: _toMap(item), id: id)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveWithCache($cls item, CacheConfig cache, {int? id}) {
    engine.writeTxn([WriteOp.put(collectionName, data: _toMap(item), id: id, cache: cache)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  void saveAll(List<$cls> items) {
    engine.writeTxn(
      items.map((item) => WriteOp.put(collectionName, data: _toMap(item))).toList(),
    );
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  $cls? updateById(int id, $cls Function($cls current) modifier) {
    final current = findById(id);
    if (current == null) return null;
    final updated = modifier(current);
    engine.writeTxn([WriteOp.put(collectionName, data: _toMap(updated), id: id)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return updated;
  }

  bool deleteById(int id) {
    engine.writeTxn([WriteOp.delete(collectionName, id: id)]);
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
    return true;
  }

  void deleteAllById(List<int> ids) {
    engine.writeTxn(
      ids.map((id) => WriteOp.delete(collectionName, id: id)).toList(),
    );
    notifier?.notifyLocal(collectionName, SyncEventType.localChange);
  }

  int deleteWhere(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    final items = findWhere(filter);
    // TODO: get IDs from findWhere when Document includes ID in data map
    return items.length;
  }

  WithProvenance<$cls>? findByIdWithProvenance(int id) {
    final doc = engine.get(collectionName, id);
    if (doc == null) return null;
    ProvenanceEnvelope? envelope;
    if (provenanceEngine != null) {
      final envelopes = provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  Stream<List<$cls>> watchAll({FilterQuery<$cls>? query, bool fireImmediately = true}) {
    late StreamController<List<$cls>> controller;
    StreamSubscription<SyncEvent>? sub;
    void requery() { if (!controller.isClosed) controller.add(findAll(query)); }
    controller = StreamController<List<$cls>>(
      onListen: () {
        if (fireImmediately) requery();
        sub = notifier?.changes.listen((event) {
          if (event.collection == '*' || event.collection == collectionName) requery();
        });
      },
      onCancel: () => sub?.cancel(),
    );
    return controller.stream;
  }

  Stream<$cls?> watchById(int id, {bool fireImmediately = true}) {
    late StreamController<$cls?> controller;
    StreamSubscription<SyncEvent>? sub;
    void requery() { if (!controller.isClosed) controller.add(findById(id)); }
    controller = StreamController<$cls?>(
      onListen: () {
        if (fireImmediately) requery();
        sub = notifier?.changes.listen((event) {
          if (event.collection == '*' || event.collection == collectionName) requery();
        });
      },
      onCancel: () => sub?.cancel(),
    );
    return controller.stream;
  }

  Stream<List<$cls>> watchWhere(
    FilterQuery<$cls> Function(FilterQuery<$cls>) filter, {
    bool fireImmediately = true,
  }) => watchAll(query: filter(FilterQuery<$cls>()), fireImmediately: fireImmediately);
''';
}

String _generateSharedMethods(String cls) {
  return '''

  List<$cls> findAll([FilterQuery<$cls>? query]) {
    final params = query?.build() ?? {};
    final docs = engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map(_fromDocument).toList();
  }

  $cls? findFirst(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    final query = filter(FilterQuery<$cls>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<$cls> findWhere(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    return findAll(filter(FilterQuery<$cls>()));
  }

  int count() => engine.count(collectionName);

  int countWhere(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    return findWhere(filter).length;
  }

  bool exists(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    return findFirst(filter) != null;
  }

  List<$cls> findPage({required int limit, int offset = 0}) {
    return findAll(FilterQuery<$cls>()..offset(offset)..limit(limit));
  }

  List<$cls> findPageWhere(
    FilterQuery<$cls> Function(FilterQuery<$cls>) filter, {
    required int limit,
    int offset = 0,
  }) {
    final query = filter(FilterQuery<$cls>())..offset(offset)..limit(limit);
    return findAll(query);
  }

  List<WithProvenance<$cls>> findAllWithProvenance([FilterQuery<$cls>? query]) {
    final params = query?.build() ?? {};
    final docs = engine.findAll(
      collectionName,
      filter: params['filter'] as Map<String, dynamic>?,
      sort: (params['sort'] as List?)?.cast<Map<String, dynamic>>(),
      offset: params['offset'] as int?,
      limit: params['limit'] as int?,
    );
    return docs.map((doc) {
      ProvenanceEnvelope? envelope;
      if (provenanceEngine != null) {
        final envelopes = provenanceEngine!.getForRecord(collectionName, doc.id);
        if (envelopes.isNotEmpty) envelope = envelopes.last;
      }
      return WithProvenance(_fromDocument(doc), envelope);
    }).toList();
  }

  List<WithProvenance<$cls>> findWhereWithProvenance(
    FilterQuery<$cls> Function(FilterQuery<$cls>) filter,
  ) {
    return findAllWithProvenance(filter(FilterQuery<$cls>()));
  }

  /// Sweep expired cached records in this collection.
  /// Returns the count of deleted records.
  int sweepExpired() => engine.sweepExpired(collectionName);
''';
}

String _generateNodeDao(ModelInfo model) {
  final cls = model.className;
  final daoName = '${cls}NodeDaoBase';

  return '''
// ── Node DAO ───────────────────────────────────────────────────
abstract class $daoName {
  GraphEngine get graphEngine;

  $cls _fromGraphNode(GraphNode node) => _\$${cls}FromMap(node.data);
  Map<String, dynamic> _toMap($cls item) => _\$${cls}ToMap(item);

  $cls addNode($cls item) {
    final node = graphEngine.addNode('${model.collectionName}', _toMap(item));
    return _fromGraphNode(node);
  }

  $cls? getNode(int id) {
    final node = graphEngine.getNode(id);
    if (node == null) return null;
    return _fromGraphNode(node);
  }

  $cls updateNode(int id, $cls item) {
    final node = graphEngine.updateNode(id, _toMap(item));
    return _fromGraphNode(node);
  }

  void deleteNode(int id, {String behaviour = 'detach'}) {
    graphEngine.deleteNode(id, behaviour: behaviour);
  }

  List<$cls> allNodes() {
    return graphEngine.allNodes()
        .where((n) => n.label == '${model.collectionName}')
        .map(_fromGraphNode)
        .toList();
  }

  int nodeCount() {
    return allNodes().length;
  }

  List<GraphEdge> edgesFrom(int nodeId) => graphEngine.edgesFrom(nodeId);
  List<GraphEdge> edgesTo(int nodeId) => graphEngine.edgesTo(nodeId);

  Map<String, List<int>> bfs(int startId, {int maxDepth = 10}) =>
      graphEngine.bfs(startId, maxDepth: maxDepth);

  Map<String, List<int>> dfs(int startId, {int maxDepth = 10}) =>
      graphEngine.dfs(startId, maxDepth: maxDepth);
}
''';
}

String _generateEdgeDao(ModelInfo model) {
  final cls = model.className;
  final daoName = '${cls}EdgeDaoBase';

  return '''
// ── Edge DAO ───────────────────────────────────────────────────
abstract class $daoName {
  GraphEngine get graphEngine;

  $cls _fromGraphEdge(GraphEdge edge) => _\$${cls}FromMap(edge.data ?? {});
  Map<String, dynamic> _toMap($cls item) => _\$${cls}ToMap(item);

  GraphEdge addEdge(int sourceId, int targetId, $cls data, {double weight = 1.0}) {
    return graphEngine.addEdge(
      '${model.collectionName}',
      sourceId,
      targetId,
      weight: weight,
      data: _toMap(data),
    );
  }

  GraphEdge? getEdge(int id) => graphEngine.getEdge(id);

  void deleteEdge(int id) => graphEngine.deleteEdge(id);

  List<GraphEdge> edgesFrom(int nodeId) =>
      graphEngine.edgesFrom(nodeId)
          .where((e) => e.label == '${model.collectionName}')
          .toList();

  List<GraphEdge> edgesTo(int nodeId) =>
      graphEngine.edgesTo(nodeId)
          .where((e) => e.label == '${model.collectionName}')
          .toList();
}
''';
}

String _generateSingletonDao(ModelInfo model) {
  final cls = model.className;
  final daoName = '${cls}DaoBase';

  return '''
// ── Singleton DAO ──────────────────────────────────────────────
abstract class $daoName {
  NoSqlEngine get engine;
  ProvenanceEngine? get provenanceEngine => null;
  String? get databaseName => null;

  String get collectionName => '${model.collectionName}';
  static const schemaName = '${model.schema ?? 'public'}';
  String get qualifiedName {
    final db = databaseName;
    if (db != null && db.isNotEmpty) return '\$db.\$schemaName.\$collectionName';
    return '\$schemaName.\$collectionName';
  }

  $cls _fromDocument(Document doc) => _\$${cls}FromMap(doc.data);
  Map<String, dynamic> _toMap($cls item) => _\$${cls}ToMap(item);

  /// Initialize the singleton with default values.
  $cls init($cls defaults) {
    final doc = engine.singletonCreate(collectionName, _toMap(defaults));
    return _fromDocument(doc);
  }

  /// Get the singleton value.
  $cls get() {
    final doc = engine.singletonGet(collectionName);
    return _fromDocument(doc);
  }

  /// Get the singleton value with its provenance envelope.
  WithProvenance<$cls> getWithProvenance() {
    final doc = engine.singletonGet(collectionName);
    ProvenanceEnvelope? envelope;
    if (provenanceEngine != null) {
      final envelopes = provenanceEngine!.getForRecord(collectionName, doc.id);
      if (envelopes.isNotEmpty) envelope = envelopes.last;
    }
    return WithProvenance(_fromDocument(doc), envelope);
  }

  /// Replace the singleton value.
  $cls put($cls item) {
    final doc = engine.singletonPut(collectionName, _toMap(item));
    return _fromDocument(doc);
  }

  /// Update the singleton using a modifier function.
  $cls update($cls Function($cls current) modifier) {
    final current = get();
    final updated = modifier(current);
    return put(updated);
  }

  /// Reset the singleton to its declared defaults.
  $cls reset() {
    final doc = engine.singletonReset(collectionName);
    return _fromDocument(doc);
  }

  /// Check if this collection is a singleton.
  bool get isSingleton => engine.isSingleton(collectionName);
}
''';
}
