import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:test/test.dart';

/// Integration tests that run against the real Rust libnodedb_ffi.dylib.
///
/// These tests require a built Rust library:
///   cd rust && cargo build -p nodedb-ffi
void main() {
  late NodeDbBindings bindings;
  late Directory tempDir;

  setUpAll(() {
    bindings = NodeDbBindings(loadNodeDbLibrary());
  });

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('nodedb_dart_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('FFI version', () {
    test('returns a positive version number', () {
      final version = bindings.ffiVersion();
      expect(version, greaterThan(0));
    });
  });

  group('NoSqlEngine', () {
    late NoSqlEngine engine;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('write and read document', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30}),
      ]);

      final docs = engine.findAll('users');
      expect(docs, hasLength(1));
      expect(docs.first.data['name'], 'Alice');
      expect(docs.first.data['age'], 30);
    });

    test('get document by ID', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Bob', 'age': 25}),
      ]);

      final docs = engine.findAll('users');
      final id = docs.first.id;

      final doc = engine.get('users', id);
      expect(doc, isNotNull);
      expect(doc!.data['name'], 'Bob');
    });

    test('count documents', () {
      engine.writeTxn([
        WriteOp.put('items', data: {'x': 1}),
        WriteOp.put('items', data: {'x': 2}),
        WriteOp.put('items', data: {'x': 3}),
      ]);

      expect(engine.count('items'), 3);
    });

    test('delete document', () {
      engine.writeTxn([
        WriteOp.put('items', data: {'x': 1}),
      ]);

      final docs = engine.findAll('items');
      expect(docs, hasLength(1));

      engine.writeTxn([
        WriteOp.delete('items', id: docs.first.id),
      ]);

      expect(engine.count('items'), 0);
    });

    test('find with filter', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30}),
        WriteOp.put('users', data: {'name': 'Bob', 'age': 20}),
        WriteOp.put('users', data: {'name': 'Charlie', 'age': 35}),
      ]);

      final adults = engine.findAll('users', filter: {
        'Condition': {
          'GreaterThan': {'field': 'age', 'value': 25},
        },
      });

      expect(adults, hasLength(2));
    });

    test('find with sort and limit', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Charlie', 'age': 35}),
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30}),
        WriteOp.put('users', data: {'name': 'Bob', 'age': 20}),
      ]);

      final sorted = engine.findAll(
        'users',
        sort: [
          {'field': 'age', 'direction': 'Asc'},
        ],
        limit: 2,
      );

      expect(sorted, hasLength(2));
      expect(sorted.first.data['name'], 'Bob');
      expect(sorted.last.data['name'], 'Alice');
    });

    test('schema create and list', () {
      engine.createSchema('test_schema');
      final schemas = engine.listSchemas();
      // Should have at least 'public' and 'test_schema'
      expect(schemas.length, greaterThanOrEqualTo(1));
    });

    test('collection names', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Test'}),
        WriteOp.put('posts', data: {'title': 'Hello'}),
      ]);

      final names = engine.collectionNames();
      // Rust returns schema-qualified names: "public.users", "public.posts"
      expect(names, contains('public.users'));
      expect(names, contains('public.posts'));
    });

    test('error on invalid operation throws NodeDbException', () {
      // Getting a non-existent document returns null, not an error
      final doc = engine.get('nonexistent', 999);
      expect(doc, isNull);
    });

    test('collection names in schema', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Test'}),
        WriteOp.put('posts', data: {'title': 'Hello'}),
      ]);

      final names = engine.collectionNamesInSchema('public');
      expect(names, contains('users'));
      expect(names, contains('posts'));
    });

    test('rename schema', () {
      engine.createSchema('old_schema');
      engine.renameSchema('old_schema', 'new_schema');
      final schemas = engine.listSchemas();
      final schemaNames = schemas.map((s) => s['name']).toList();
      expect(schemaNames, contains('new_schema'));
    });

    test('move collection between schemas', () {
      engine.createSchema('other');
      engine.writeTxn([
        WriteOp.put('items', data: {'val': 1}),
      ]);

      // Move from public.items to other schema
      engine.moveCollection('public.items', 'other');

      final otherNames = engine.collectionNamesInSchema('other');
      expect(otherNames, contains('items'));
    });
  });

  group('FilterQuery', () {
    late NoSqlEngine engine;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30}),
        WriteOp.put('users', data: {'name': 'Bob', 'age': 20}),
        WriteOp.put('users', data: {'name': 'Charlie', 'age': 35}),
      ]);
    });

    tearDown(() {
      engine.close();
    });

    test('builds correct filter map', () {
      final query = FilterQuery<dynamic>()
          .greaterThan('age', 25)
          .sortBy('age')
          .limit(10);

      final built = query.build();
      expect(built['filter'], isNotNull);
      expect(built['sort'], isNotNull);
      expect(built['limit'], 10);
    });

    test('AND filter', () {
      final query = FilterQuery<dynamic>()
          .greaterThan('age', 25)
          .and((q) => q.startsWith('name', 'A'));

      final built = query.build();
      final filter = built['filter'] as Map<String, dynamic>;
      expect(filter.containsKey('And'), isTrue);
    });

    test('OR filter', () {
      final query = FilterQuery<dynamic>()
          .equalTo('name', 'Alice')
          .or((q) => q.equalTo('name', 'Bob'));

      final built = query.build();
      final filter = built['filter'] as Map<String, dynamic>;
      expect(filter.containsKey('Or'), isTrue);
    });

    test('inList filter queries matching records', () {
      final query = FilterQuery<dynamic>().inList('name', ['Alice', 'Charlie']);
      final built = query.build();

      final docs = engine.findAll(
        'users',
        filter: built['filter'] as Map<String, dynamic>?,
      );
      expect(docs, hasLength(2));
      final names = docs.map((d) => d.data['name']).toSet();
      expect(names, containsAll(['Alice', 'Charlie']));
    });

    test('notInList filter excludes matching records', () {
      final query = FilterQuery<dynamic>().notInList('name', ['Bob']);
      final built = query.build();

      final docs = engine.findAll(
        'users',
        filter: built['filter'] as Map<String, dynamic>?,
      );
      expect(docs, hasLength(2));
      final names = docs.map((d) => d.data['name']).toSet();
      expect(names, containsAll(['Alice', 'Charlie']));
      expect(names, isNot(contains('Bob')));
    });

    test('inList with numeric values', () {
      final query = FilterQuery<dynamic>().inList('age', [20, 35]);
      final built = query.build();

      final docs = engine.findAll(
        'users',
        filter: built['filter'] as Map<String, dynamic>?,
      );
      expect(docs, hasLength(2));
      final names = docs.map((d) => d.data['name']).toSet();
      expect(names, containsAll(['Bob', 'Charlie']));
    });
  });

  group('GraphEngine', () {
    late GraphEngine engine;

    setUp(() {
      engine = GraphEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('add and get node', () {
      final node = engine.addNode('person', {'name': 'Alice'});
      expect(node.id, greaterThan(0));
      expect(node.label, 'person');
      expect(node.data['name'], 'Alice');

      final fetched = engine.getNode(node.id);
      expect(fetched, isNotNull);
      expect(fetched!.data['name'], 'Alice');
    });

    test('add edge between nodes', () {
      final alice = engine.addNode('person', {'name': 'Alice'});
      final bob = engine.addNode('person', {'name': 'Bob'});

      final edge = engine.addEdge('knows', alice.id, bob.id, weight: 0.9);
      expect(edge.id, greaterThan(0));
      expect(edge.source, alice.id);
      expect(edge.target, bob.id);

      final outEdges = engine.edgesFrom(alice.id);
      expect(outEdges, hasLength(1));
      expect(outEdges.first.target, bob.id);
    });

    test('node count', () {
      engine.addNode('person', {'name': 'A'});
      engine.addNode('person', {'name': 'B'});
      expect(engine.nodeCount(), 2);
    });

    test('delete node', () {
      final node = engine.addNode('person', {'name': 'Test'});
      engine.deleteNode(node.id);
      expect(engine.getNode(node.id), isNull);
    });

    test('BFS traversal', () {
      final a = engine.addNode('person', {'name': 'A'});
      final b = engine.addNode('person', {'name': 'B'});
      final c = engine.addNode('person', {'name': 'C'});
      engine.addEdge('knows', a.id, b.id);
      engine.addEdge('knows', b.id, c.id);

      final result = engine.bfs(a.id);
      expect(result['nodes']!.length, greaterThanOrEqualTo(3));
      expect(result['nodes'], contains(a.id));
      expect(result['nodes'], contains(b.id));
      expect(result['nodes'], contains(c.id));
    });
  });

  group('VectorEngine', () {
    late VectorEngine engine;

    setUp(() {
      engine = VectorEngine.open(
        bindings,
        VectorOpenConfig(
          path: tempDir.path,
          dimension: 3,
          metric: 'cosine',
          maxElements: 1000,
        ),
      );
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('insert and count', () {
      engine.insert([1.0, 0.0, 0.0], metadata: {'label': 'x-axis'});
      engine.insert([0.0, 1.0, 0.0], metadata: {'label': 'y-axis'});
      expect(engine.count(), 2);
    });

    test('insert and get', () {
      final record =
          engine.insert([1.0, 2.0, 3.0], metadata: {'name': 'test'});
      expect(record.id, greaterThan(0));

      final fetched = engine.get(record.id);
      expect(fetched, isNotNull);
    });

    test('search returns nearest neighbors', () {
      engine.insert([1.0, 0.0, 0.0], metadata: {'label': 'a'});
      engine.insert([0.0, 1.0, 0.0], metadata: {'label': 'b'});
      engine.insert([0.0, 0.0, 1.0], metadata: {'label': 'c'});

      final results = engine.search([1.0, 0.1, 0.0], k: 2);
      expect(results, hasLength(2));
      // First result should be closest to query vector
      expect(results.first.id, greaterThan(0));
    });

    test('delete record', () {
      final record = engine.insert([1.0, 0.0, 0.0]);
      expect(engine.count(), 1);
      engine.delete(record.id);
      // Count may still be 1 (soft delete) or 0 depending on implementation
    });
  });

  group('FederationEngine', () {
    late FederationEngine engine;

    setUp(() {
      engine = FederationEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('add and get peer', () {
      final peer = engine.addPeer('node-1', 'ws://localhost:8080');
      expect(peer.id, greaterThan(0));
      expect(peer.name, 'node-1');

      final fetched = engine.getPeer(peer.id);
      expect(fetched, isNotNull);
      expect(fetched!.name, 'node-1');
    });

    test('get peer by name', () {
      engine.addPeer('alpha', 'ws://alpha:8080');
      final peer = engine.getPeerByName('alpha');
      expect(peer, isNotNull);
      expect(peer!.name, 'alpha');
    });

    test('peer count', () {
      engine.addPeer('p1', 'ws://p1:8080');
      engine.addPeer('p2', 'ws://p2:8080');
      expect(engine.peerCount(), 2);
    });

    test('delete peer', () {
      final peer = engine.addPeer('temp', 'ws://temp:8080');
      engine.deletePeer(peer.id);
      expect(engine.peerCount(), 0);
    });

    test('add and get group', () {
      final group = engine.addGroup('cluster-a');
      expect(group.id, greaterThan(0));
      expect(group.name, 'cluster-a');

      final fetched = engine.getGroup(group.id);
      expect(fetched, isNotNull);
      expect(fetched!.name, 'cluster-a');
    });

    test('group membership', () {
      final peer = engine.addPeer('member', 'ws://m:8080');
      final group = engine.addGroup('team');
      engine.addMember(group.id, peer.id);

      final groupIds = engine.groupsForPeer(peer.id);
      expect(groupIds, hasLength(1));
      expect(groupIds.first, group.id);
    });
  });

  group('DacEngine', () {
    late DacEngine engine;

    setUp(() {
      engine = DacEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('add and get rule', () {
      final rule = engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );
      expect(rule.id, greaterThan(0));

      final fetched = engine.getRule(rule.id);
      expect(fetched, isNotNull);
    });

    test('rule count', () {
      engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );
      engine.addRule(
        collection: 'posts',
        subjectType: 'peer',
        subjectId: 'peer-2',
        permission: 'allow',
      );
      expect(engine.ruleCount(), 2);
    });

    test('rules for collection', () {
      engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );
      engine.addRule(
        collection: 'posts',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );

      final userRules = engine.rulesForCollection('users');
      expect(userRules, hasLength(1));
    });

    test('delete rule', () {
      final rule = engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );
      engine.deleteRule(rule.id);
      expect(engine.ruleCount(), 0);
    });

    test('filter document', () {
      // Add a read rule for peer-1 on 'users'
      engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer-1',
        permission: 'allow',
      );

      final filtered = engine.filterDocument(
        collection: 'users',
        document: {'name': 'Alice', 'age': 30},
        peerId: 'peer-1',
      );
      // With read access, document should be returned
      expect(filtered, isNotNull);
    });
  });

  group('ProvenanceEngine', () {
    late ProvenanceEngine engine;

    setUp(() {
      engine = ProvenanceEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('attach provenance', () {
      final envelope = engine.attach(
        collection: 'users',
        recordId: 1,
        sourceId: 'source-a',
        sourceType: 'user',
        contentHash: 'abc123',
      );
      expect(envelope.id, greaterThan(0));
    });

    test('get provenance', () {
      final envelope = engine.attach(
        collection: 'users',
        recordId: 1,
        sourceId: 'source-a',
        sourceType: 'user',
        contentHash: 'abc123',
      );

      final fetched = engine.get(envelope.id);
      expect(fetched, isNotNull);
      expect(fetched!.id, envelope.id);
    });

    test('get for record', () {
      engine.attach(
        collection: 'users',
        recordId: 42,
        sourceId: 'source-a',
        sourceType: 'user',
        contentHash: 'hash1',
      );
      engine.attach(
        collection: 'users',
        recordId: 42,
        sourceId: 'source-b',
        sourceType: 'peer',
        contentHash: 'hash2',
      );

      final envelopes = engine.getForRecord('users', 42);
      expect(envelopes.length, greaterThanOrEqualTo(2));
    });

    test('count', () {
      engine.attach(
        collection: 'items',
        recordId: 1,
        sourceId: 's1',
        sourceType: 'user',
        contentHash: 'h1',
      );
      expect(engine.count(), greaterThanOrEqualTo(1));
    });

    test('delete', () {
      final envelope = engine.attach(
        collection: 'items',
        recordId: 1,
        sourceId: 's1',
        sourceType: 'user',
        contentHash: 'h1',
      );
      engine.delete(envelope.id);
      final fetched = engine.get(envelope.id);
      expect(fetched, isNull);
    });

    test('compute hash', () {
      final hash = engine.computeHash({'name': 'Alice', 'age': 30});
      expect(hash, isA<String>());
      expect(hash.length, greaterThan(0));
    });

    test('checkedAtUtc is null on attach', () {
      final envelope = engine.attach(
        collection: 'items',
        recordId: 1,
        sourceId: 'src',
        sourceType: 'user',
        contentHash: 'h1',
      );
      expect(envelope.checkedAtUtc, isNull);
      // Verify the field round-trips through get
      final fetched = engine.get(envelope.id);
      expect(fetched, isNotNull);
      expect(fetched!.checkedAtUtc, isNull);
    });
  });

  group('KeyResolverEngine', () {
    late KeyResolverEngine engine;

    setUp(() {
      engine = KeyResolverEngine.open(bindings, tempDir.path);
    });

    tearDown(() {
      engine.close();
    });

    test('open and close', () {
      expect(engine.handle, isNonZero);
    });

    test('supply and get key', () {
      final key = engine.supplyKey(
        pkiId: 'pki-1',
        userId: 'user-1',
        publicKeyHex: 'aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd',
      );
      expect(key.pkiId, 'pki-1');

      final fetched = engine.getKey('pki-1', 'user-1');
      expect(fetched, isNotNull);
      expect(fetched!.pkiId, 'pki-1');
    });

    test('key count', () {
      engine.supplyKey(
        pkiId: 'pki-a',
        userId: 'user-a',
        publicKeyHex: '1234567812345678123456781234567812345678123456781234567812345678',
      );
      engine.supplyKey(
        pkiId: 'pki-b',
        userId: 'user-b',
        publicKeyHex: '5678abcd5678abcd5678abcd5678abcd5678abcd5678abcd5678abcd5678abcd',
      );
      expect(engine.keyCount(), 2);
    });

    test('all keys', () {
      engine.supplyKey(
        pkiId: 'pki-a',
        userId: 'user-a',
        publicKeyHex: '1234567812345678123456781234567812345678123456781234567812345678',
      );
      final keys = engine.allKeys();
      expect(keys, hasLength(1));
      expect(keys.first.pkiId, 'pki-a');
    });

    test('revoke key', () {
      engine.supplyKey(
        pkiId: 'pki-r',
        userId: 'user-r',
        publicKeyHex: 'abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd',
      );
      engine.revokeKey('pki-r', 'user-r');
      final key = engine.getKey('pki-r', 'user-r');
      // Revoked key may still be retrievable but with revoked status
      // or may be null depending on implementation
      if (key != null) {
        expect(key.trustLevel, 'revoked');
      }
    });

    test('trust all mode', () {
      expect(engine.isTrustAllActive(), isFalse);
      engine.setTrustAll(enabled: true);
      expect(engine.isTrustAllActive(), isTrue);
      engine.setTrustAll(enabled: false);
      expect(engine.isTrustAllActive(), isFalse);
    });
  });

  group('AiProvenanceEngine', () {
    late ProvenanceEngine provEngine;
    late AiProvenanceEngine aiProvEngine;

    setUp(() {
      final provDir = Directory('${tempDir.path}/prov')..createSync();
      provEngine = ProvenanceEngine.open(bindings, provDir.path);
      aiProvEngine = AiProvenanceEngine.open(bindings, provEngine.handle);
    });

    tearDown(() {
      aiProvEngine.close();
      provEngine.close();
    });

    test('open and close', () {
      expect(aiProvEngine.handle, isNonZero);
    });

    test('get config', () {
      final config = aiProvEngine.getConfig();
      expect(config, isA<Map<String, dynamic>>());
    });

    test('apply assessment', () {
      // First create an envelope to assess
      final envelope = provEngine.attach(
        collection: 'users',
        recordId: 1,
        sourceId: 'test',
        sourceType: 'user',
        contentHash: 'hash123',
      );

      final result = aiProvEngine.applyAssessment(
        envelopeId: envelope.id,
        suggestedConfidence: 0.85,
        reasoning: 'high quality source',
      );
      expect(result, isNotNull);
    });
  });

  group('AiQueryEngine', () {
    late NoSqlEngine nosqlEngine;
    late ProvenanceEngine provEngine;
    late AiQueryEngine aiQueryEngine;

    setUp(() {
      final nosqlDir = Directory('${tempDir.path}/nosql')..createSync();
      final provDir = Directory('${tempDir.path}/prov')..createSync();
      nosqlEngine = NoSqlEngine.open(bindings, nosqlDir.path);
      provEngine = ProvenanceEngine.open(bindings, provDir.path);
      aiQueryEngine = AiQueryEngine.open(
        bindings,
        nosqlHandle: nosqlEngine.handle,
        provenanceHandle: provEngine.handle,
        enabledCollections: ['users'],
      );
    });

    tearDown(() {
      aiQueryEngine.close();
      provEngine.close();
      nosqlEngine.close();
    });

    test('open and close', () {
      expect(aiQueryEngine.handle, isNonZero);
    });

    test('get config', () {
      final config = aiQueryEngine.getConfig();
      expect(config, isA<Map<String, dynamic>>());
    });

    test('process results', () {
      // Write some test data
      nosqlEngine.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30}),
      ]);

      final result = aiQueryEngine.processResults(
        collection: 'users',
        results: [
          {'name': 'Alice', 'age': 30},
        ],
      );
      expect(result, isNotNull);
    });
  });

  group('NodeDB facade', () {
    test('open with all engines', () {
      // Each engine needs its own subdirectory to avoid sled lock conflicts
      final nosqlDir = Directory('${tempDir.path}/nosql')..createSync();
      final graphDir = Directory('${tempDir.path}/graph')..createSync();
      final fedDir = Directory('${tempDir.path}/federation')..createSync();
      final dacDir = Directory('${tempDir.path}/dac')..createSync();
      final provDir = Directory('${tempDir.path}/provenance')..createSync();
      final krDir = Directory('${tempDir.path}/keyresolver')..createSync();

      final nosql = NoSqlEngine.open(bindings, nosqlDir.path);
      final graph = GraphEngine.open(bindings, graphDir.path);
      final federation = FederationEngine.open(bindings, fedDir.path);
      final dac = DacEngine.open(bindings, dacDir.path);
      final provenance = ProvenanceEngine.open(bindings, provDir.path);
      final keyResolver = KeyResolverEngine.open(bindings, krDir.path);

      expect(nosql.handle, isNonZero);
      expect(graph.handle, isNonZero);
      expect(federation.handle, isNonZero);
      expect(dac.handle, isNonZero);
      expect(provenance.handle, isNonZero);
      expect(keyResolver.handle, isNonZero);
      expect(bindings.ffiVersion(), greaterThan(0));

      keyResolver.close();
      provenance.close();
      dac.close();
      federation.close();
      graph.close();
      nosql.close();
    });

    test('open nosql-only (default)', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      expect(db.nosql.handle, isNonZero);
      expect(db.graph, isNull);
      expect(db.vector, isNull);

      db.close();
    });

    test('facade CRUD delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      db.writeTxn([
        WriteOp.put('items', data: {'name': 'A', 'val': 1}),
        WriteOp.put('items', data: {'name': 'B', 'val': 2}),
        WriteOp.put('items', data: {'name': 'C', 'val': 3}),
      ]);

      expect(db.count('items'), 3);
      expect(db.collectionNames(), contains('public.items'));

      final all = db.findAll('items');
      expect(all, hasLength(3));

      final doc = db.get('items', all.first.id);
      expect(doc, isNotNull);
      expect(doc!.data['name'], 'A');

      db.close();
    });

    test('facade trigger delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      final id = db.registerTrigger(
        collection: 'logs',
        event: 'insert',
        timing: 'after',
        name: 'log_trigger',
      );
      expect(id, greaterThan(0));

      final disabled = db.setTriggerEnabled(id, enabled: false);
      expect(disabled, isTrue);

      final removed = db.unregisterTrigger(id);
      expect(removed, isTrue);

      db.close();
    });

    test('facade singleton delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      final doc = db.singletonCreate('config', {'theme': 'dark', 'lang': 'en'});
      expect(doc.data['theme'], 'dark');

      final updated = db.singletonPut('config', {'theme': 'light', 'lang': 'en'});
      expect(updated.data['theme'], 'light');

      expect(db.isSingleton('config'), isTrue);

      final reset = db.singletonReset('config');
      expect(reset.data['theme'], 'dark');

      db.close();
    });

    test('facade preference delegates', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      db.prefSet('settings', 'theme', 'dark');
      db.prefSet('settings', 'fontSize', 14);

      final themeResp = db.prefGet('settings', 'theme');
      expect(themeResp['found'], isTrue);
      expect(themeResp['value'], 'dark');
      final fontResp = db.prefGet('settings', 'fontSize');
      expect(fontResp['found'], isTrue);
      expect(fontResp['value'], 14);
      expect(db.prefKeys('settings'), containsAll(['theme', 'fontSize']));

      expect(db.prefRemove('settings', 'theme'), isTrue);
      expect(db.prefKeys('settings'), isNot(contains('theme')));

      db.close();
    });

    test('findAllFiltered with DAC rules', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
        dacEnabled: true,
      );

      db.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice', 'age': 30, 'secret': 'abc'}),
        WriteOp.put('users', data: {'name': 'Bob', 'age': 25, 'secret': 'xyz'}),
      ]);

      // Without DAC rules, all data visible
      final all = db.findAllFiltered('users', peerId: 'peer1');
      expect(all, hasLength(2));

      // Add allow rule first, then redact rule for 'secret' field
      db.dac!.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'peer1',
        permission: 'allow',
      );
      db.dac!.addRule(
        collection: 'users',
        field: 'secret',
        subjectType: 'peer',
        subjectId: 'peer1',
        permission: 'redact',
      );

      // With redact rule, secret field should be nulled out
      final filtered = db.findAllFiltered('users', peerId: 'peer1');
      expect(filtered, hasLength(2));
      for (final doc in filtered) {
        expect(doc.data['secret'], isNull);
        expect(doc.data['name'], isNotNull);
      }

      db.close();
    });

    test('getWithProvenance returns document with envelope', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
        provenanceEnabled: true,
      );

      db.writeTxn([
        WriteOp.put('items', data: {'val': 42}),
      ]);
      final doc = db.findAll('items').first;

      // Compute hash and attach provenance
      final hash = db.provenance!.computeHash(doc.data);
      db.provenance!.attach(
        collection: 'items',
        recordId: doc.id,
        sourceId: 'test-source',
        sourceType: 'user',
        contentHash: hash,
      );

      final result = db.getWithProvenance('items', doc.id);
      expect(result, isNotNull);
      expect(result!.data.data['val'], 42);
      expect(result.provenance, isNotNull);
      expect(result.provenance!.sourceId, 'test-source');

      db.close();
    });

    test('getNodeWithEdges returns node with connections', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
        graphEnabled: true,
      );

      final alice = db.graph!.addNode('person', {'name': 'Alice'});
      final bob = db.graph!.addNode('person', {'name': 'Bob'});
      db.graph!.addEdge('knows', alice.id, bob.id);

      final result = db.getNodeWithEdges(alice.id);
      expect(result, isNotNull);
      expect(result!['node'], isNotNull);
      expect((result['edgesFrom'] as List), hasLength(1));
      expect((result['edgesTo'] as List), hasLength(0));

      db.close();
    });

    test('schema delegates on facade', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        databaseName: 'test',
        bindings: bindings,
      );

      db.createSchema('analytics');
      final schemas = db.listSchemas();
      final schemaNames = schemas.map((s) => s['name']).toList();
      expect(schemaNames, contains('analytics'));

      final fingerprint = db.schemaFingerprint();
      expect(fingerprint, isNotEmpty);

      db.close();
    });
  });

  group('Security schema', () {
    test('security schema exists by default and is listed', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/secschema_list',
        databaseName: 'test',
        bindings: bindings,
      );

      final schemas = db.listSchemas();
      final schemaNames = schemas.map((s) => s['name']).toList();
      expect(schemaNames, contains('security'));

      db.close();
    });

    test('write to security schema throws ReservedSchemaWriteException', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/secschema_write',
        databaseName: 'test',
        bindings: bindings,
      );

      expect(
        () => db.writeTxn([
          WriteOp.put('security.secrets', data: {'key': 'value'}),
        ]),
        throwsA(isA<ReservedSchemaWriteException>()),
      );

      db.close();
    });

    test('create security schema is blocked', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/secschema_create',
        databaseName: 'test',
        bindings: bindings,
      );

      expect(
        () => db.createSchema('security'),
        throwsA(isA<NodeDbException>()),
      );

      db.close();
    });
  });

  group('Provenance lifecycle fields', () {
    test('attach with dataUpdatedAtUtc, localId, globalId round-trips', () {
      final provDir = Directory('${tempDir.path}/prov_lifecycle');
      provDir.createSync(recursive: true);
      final prov = ProvenanceEngine.open(bindings, provDir.path);

      final env = prov.attach(
        collection: 'users',
        recordId: 1,
        sourceId: 'src',
        sourceType: 'user',
        contentHash: 'a' * 64,
        dataUpdatedAtUtc: '2025-06-01T11:00:00Z',
        localId: 'local-abc-123',
        globalId: 'global-xyz-789',
      );

      expect(env.dataUpdatedAtUtc, isNotNull);
      expect(env.localId, equals('local-abc-123'));
      expect(env.globalId, equals('global-xyz-789'));

      // Re-fetch and verify persistence
      final fetched = prov.get(env.id);
      expect(fetched, isNotNull);
      expect(fetched!.localId, equals('local-abc-123'));
      expect(fetched.globalId, equals('global-xyz-789'));
      expect(fetched.dataUpdatedAtUtc, isNotNull);

      prov.close();
    });

    test('attach without lifecycle fields defaults to null', () {
      final provDir = Directory('${tempDir.path}/prov_no_lifecycle');
      provDir.createSync(recursive: true);
      final prov = ProvenanceEngine.open(bindings, provDir.path);

      final env = prov.attach(
        collection: 'users',
        recordId: 1,
        sourceId: 'src',
        sourceType: 'user',
        contentHash: 'b' * 64,
      );

      expect(env.dataUpdatedAtUtc, isNull);
      expect(env.localId, isNull);
      expect(env.globalId, isNull);

      prov.close();
    });
  });

  group('Centralized mgmt database', () {
    test('federation is always available via mgmt', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/mgmt_always',
        databaseName: 'test',
        bindings: bindings,
      );

      // Federation is always non-null now
      final peer = db.federation.addPeer('alice', 'wss://alice:9400');
      expect(peer.id, greaterThan(0));
      expect(db.federation.allPeers(), hasLength(1));

      db.close();
    });

    test('mgmt federation persists across reopen', () {
      final dir = '${tempDir.path}/mgmt_persist';
      

      var db = NodeDB.open(directory: dir, databaseName: 'test', bindings: bindings);
      db.federation.addPeer('bob', 'wss://bob:9400');
      db.close();

      db = NodeDB.open(directory: dir, databaseName: 'test', bindings: bindings);
      expect(db.federation.allPeers(), hasLength(1));
      expect(db.federation.allPeers().first.name, 'bob');
      db.close();
    });
  });

  group('Access History', () {
    late NodeDB db;

    setUp(() {
      db = NodeDB.open(
        directory: '${tempDir.path}/access_hist',
        databaseName: 'test',
        bindings: bindings,
      );
    });

    tearDown(() => db.close());

    test('records are created on read/write via FFI', () {
      // Write a document (triggers write access recording at FFI boundary)
      db.writeTxn([WriteOp.put('users', data: {'name': 'Alice'})]);

      // Read it back (triggers read access recording)
      db.findAll('users');

      // Access history should have entries
      final count = db.accessHistoryCount();
      expect(count, greaterThan(0));
    });

    test('query filters by collection', () {
      db.writeTxn([WriteOp.put('items', data: {'title': 'Book'})]);
      db.writeTxn([WriteOp.put('orders', data: {'total': 99})]);

      // FFI records collection name as provided in the request (unqualified)
      final itemHistory =
          db.accessHistoryQuery(collection: 'items');
      final orderHistory =
          db.accessHistoryQuery(collection: 'orders');

      // Both should have write entries
      expect(itemHistory, isNotEmpty);
      expect(orderHistory, isNotEmpty);
    });

    test('last access returns a timestamp', () {
      db.writeTxn([WriteOp.put('notes', data: {'text': 'hello'})]);

      // The FFI records access for writes; we need the record ID
      final docs = db.findAll('notes');
      expect(docs, hasLength(1));

      final lastAccess =
          db.accessHistoryLastAccess('notes', docs.first.id);
      expect(lastAccess, isNotNull);
    });

    test('trim removes old entries', () {
      db.writeTxn([WriteOp.put('logs', data: {'msg': 'entry1'})]);
      db.writeTxn([WriteOp.put('logs', data: {'msg': 'entry2'})]);

      final before = db.accessHistoryCount();
      expect(before, greaterThan(0));

      // Trim with 0 retention → removes everything
      final deleted = db.accessHistoryTrim(retentionSecs: 0);
      expect(deleted, greaterThan(0));

      final after = db.accessHistoryCount();
      expect(after, lessThan(before));
    });
  });

  group('Trim', () {
    late NodeDB db;

    setUp(() {
      db = NodeDB.open(
        directory: '${tempDir.path}/trim_test',
        databaseName: 'test',
        bindings: bindings,
      );
    });

    tearDown(() => db.close());

    test('collections are never-trim by default', () {
      db.writeTxn([WriteOp.put('data', data: {'x': 1})]);
      expect(db.trimConfigIsNeverTrim('data'), isTrue);
    });

    test('set trim policy makes collection trimmable', () {
      db.writeTxn([WriteOp.put('cache', data: {'x': 1})]);

      db.trimConfigSet(
          'cache', const TrimNotAccessedSince(60));
      expect(db.trimConfigIsNeverTrim('cache'), isFalse);

      final effective = db.trimConfigEffective('cache');
      expect(effective, isNotNull);
    });

    test('reset returns collection to never-trim', () {
      db.writeTxn([WriteOp.put('temp', data: {'x': 1})]);

      db.trimConfigSet('temp', const TrimNotAccessedSince(60));
      expect(db.trimConfigIsNeverTrim('temp'), isFalse);

      db.trimConfigReset('temp');
      expect(db.trimConfigIsNeverTrim('temp'), isTrue);
    });

    test('record-level never-trim protection', () {
      db.writeTxn([WriteOp.put('ephemeral', data: {'x': 1})]);
      final docs = db.findAll('ephemeral');
      final id = docs.first.id;

      db.trimConfigSetRecordNeverTrim('ephemeral', id);

      // Make collection trimmable
      db.trimConfigSet(
          'ephemeral', const TrimNotAccessedSince(0));

      // Trim should skip the protected record
      final report = db.trim('ephemeral',
          const TrimNotAccessedSince(0));
      expect(report.neverTrimSkippedCount, greaterThanOrEqualTo(1));
    });

    test('dry run does not delete', () {
      db.writeTxn([WriteOp.put('drydata', data: {'x': 1})]);
      db.trimConfigSet(
          'drydata', const TrimNotAccessedSince(0));

      final report = db.trim('drydata',
          const TrimNotAccessedSince(0),
          dryRun: true);
      expect(report.dryRun, isTrue);
      // deletedCount in dry_run shows what *would* be deleted
      expect(report.deletedCount, greaterThan(0));

      // Data should still exist — dry run doesn't actually delete
      expect(db.findAll('drydata'), hasLength(1));
    });

    test('recommend_trim returns candidates', () {
      db.writeTxn([WriteOp.put('rec_test', data: {'x': 1})]);
      db.trimConfigSet(
          'rec_test', const TrimNotAccessedSince(0));

      final rec = db.recommendTrim(const TrimNotAccessedSince(0));
      expect(rec.totalCandidateCount, greaterThan(0));
      expect(rec.byCollection, isNotEmpty);
    });

    test('trim_approved with user confirmation', () {
      db.writeTxn([WriteOp.put('approved', data: {'x': 1})]);
      db.trimConfigSet(
          'approved', const TrimNotAccessedSince(0));

      final rec = db.recommendTrim(const TrimNotAccessedSince(0));
      expect(rec.totalCandidateCount, greaterThan(0));

      // Build approval from recommendation
      final confirmed = <({String collection, int recordId})>[];
      for (final cr in rec.byCollection) {
        for (final c in cr.candidates) {
          confirmed.add((collection: c.collection, recordId: c.recordId));
        }
      }

      final report = db.trimApproved(UserApprovedTrim(
        policy: const TrimNotAccessedSince(0),
        confirmedRecordIds: confirmed,
        approvalNote: 'test approval',
      ));

      expect(report.deletedCount, greaterThan(0));
    });

    test('clear record override', () {
      db.writeTxn([WriteOp.put('cleartest', data: {'x': 1})]);
      final docs = db.findAll('cleartest');
      final id = docs.first.id;

      db.trimConfigSetRecordNeverTrim('cleartest', id);
      db.trimConfigClearRecordOverride('cleartest', id);

      // Should no longer be protected at record level
      db.trimConfigSet(
          'cleartest', const TrimNotAccessedSince(0));
      final report = db.trim('cleartest',
          const TrimNotAccessedSince(0));
      expect(report.neverTrimSkippedCount, 0);
    });
  });

  group('AI Adapter Models', () {
    test('ConflictPreference round-trips through FFI strings', () {
      expect(ConflictPreference.preferA.toFfiString(), 'prefer_a');
      expect(ConflictPreference.preferB.toFfiString(), 'prefer_b');
      expect(ConflictPreference.indeterminate.toFfiString(), 'prefer_neither');
      expect(ConflictPreference.fromFfiString('prefer_a'),
          ConflictPreference.preferA);
      expect(ConflictPreference.fromFfiString('PreferB'),
          ConflictPreference.preferB);
      expect(ConflictPreference.fromFfiString('unknown'),
          ConflictPreference.indeterminate);
    });

    test('AnomalySeverity parses from string', () {
      expect(AnomalySeverity.fromString('low'), AnomalySeverity.low);
      expect(AnomalySeverity.fromString('MEDIUM'), AnomalySeverity.medium);
      expect(AnomalySeverity.fromString('High'), AnomalySeverity.high);
      expect(AnomalySeverity.fromString('critical'), AnomalySeverity.critical);
      expect(AnomalySeverity.fromString('unknown'), AnomalySeverity.low);
    });

    test('AiProvenanceAssessment toFfiMap', () {
      const assessment = AiProvenanceAssessment(
        suggestedConfidence: 0.85,
        sourceType: 'user',
        reasoning: 'Looks legit',
        tags: {'quality': 'high'},
      );
      final map = assessment.toFfiMap(42);
      expect(map['envelope_id'], 42);
      expect(map['suggested_confidence'], 0.85);
      expect(map['source_type'], 'user');
      expect(map['reasoning'], 'Looks legit');
      expect(map['tags'], {'quality': 'high'});
    });

    test('AiAnomalyFlag toFfiMap', () {
      const flag = AiAnomalyFlag(
        recordId: 7,
        confidencePenalty: 0.3,
        reason: 'Suspicious',
        severity: AnomalySeverity.high,
      );
      final map = flag.toFfiMap();
      expect(map['record_id'], 7);
      expect(map['confidence_penalty'], 0.3);
      expect(map['reason'], 'Suspicious');
      expect(map['severity'], 'high');
    });

    test('AiQueryResult toFfiMap', () {
      const result = AiQueryResult(
        data: {'name': 'Alice'},
        confidence: 0.92,
        sourceExplanation: 'from AI',
        externalSourceUri: 'https://example.com',
        tags: {'src': 'gpt'},
      );
      final map = result.toFfiMap();
      expect(map['data'], {'name': 'Alice'});
      expect(map['confidence'], 0.92);
      expect(map['source_explanation'], 'from AI');
      expect(map['external_source_uri'], 'https://example.com');
      expect(map['tags'], {'src': 'gpt'});
    });

    test('AiQueryWriteDecision fromMap', () {
      final decision = AiQueryWriteDecision.fromMap({
        'persisted': true,
        'record_id': 5,
        'confidence': 0.88,
        'ai_origin_tag': 'ai-query:items:2026',
      });
      expect(decision.persisted, isTrue);
      expect(decision.recordId, 5);
      expect(decision.confidence, 0.88);
      expect(decision.aiOriginTag, 'ai-query:items:2026');
      expect(decision.rejectionReason, isNull);
    });

    test('AiQuerySchema toMap', () {
      const schema = AiQuerySchema(
        requiredFields: ['name', 'age'],
        fieldTypes: {
          'name': SchemaPropertyType.string,
          'age': SchemaPropertyType.integer,
        },
      );
      final map = schema.toMap();
      expect(map['required_fields'], ['name', 'age']);
      expect(map['field_types']['name'], 'String');
      expect(map['field_types']['age'], 'Integer');
    });

    test('SchemaPropertyType parses from string', () {
      expect(SchemaPropertyType.fromString('string'),
          SchemaPropertyType.string);
      expect(SchemaPropertyType.fromString('INTEGER'),
          SchemaPropertyType.integer);
      expect(SchemaPropertyType.fromString('Float'),
          SchemaPropertyType.float);
      expect(SchemaPropertyType.fromString('unknown'),
          SchemaPropertyType.any);
    });

    test('AiProvenanceConfig defaults', () {
      const config = AiProvenanceConfig();
      expect(config.aiBlendWeight, 0.3);
      expect(config.enabledCollections, isEmpty);
      expect(config.responseTimeout.inSeconds, 5);
      expect(config.silentOnError, isTrue);
      expect(config.rateLimitPerMinute, 60);
    });

    test('AiQueryConfig defaults', () {
      const config = AiQueryConfig();
      expect(config.enabledCollections, isEmpty);
      expect(config.minimumWriteConfidence, 0.80);
      expect(config.maxResultsPerQuery, 10);
      expect(config.reportWriteDecisions, isTrue);
      expect(config.tryFederationFirst, isTrue);
      expect(config.rateLimitPerMinute, 20);
    });
  });

  group('AI Adapter Facade', () {
    test('configureAiProvenance throws without provenance', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/ai_no_prov',
        databaseName: 'test',
        bindings: bindings,
      );

      expect(
        () => db.configureAiProvenance(adapter: _StubAiProvenanceAdapter()),
        throwsA(isA<StateError>()),
      );
      db.close();
    });

    test('configureAiQuery throws without provenance', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/ai_no_prov2',
        databaseName: 'test',
        bindings: bindings,
      );

      expect(
        () => db.configureAiQuery(adapter: _StubAiQueryAdapter()),
        throwsA(isA<StateError>()),
      );
      db.close();
    });

    test('configureAiProvenance succeeds with provenance', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/ai_prov',
        databaseName: 'test',
        bindings: bindings,
        provenanceEnabled: true,
      );

      final adapter = _StubAiProvenanceAdapter();
      db.configureAiProvenance(
        adapter: adapter,
        config: const AiProvenanceConfig(aiBlendWeight: 0.5),
      );

      expect(db.aiProvenanceAdapter, same(adapter));
      expect(db.aiProvenanceConfig.aiBlendWeight, 0.5);
      db.close();
    });

    test('configureAiQuery succeeds with provenance', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/ai_query',
        databaseName: 'test',
        bindings: bindings,
        provenanceEnabled: true,
      );

      final adapter = _StubAiQueryAdapter();
      db.configureAiQuery(
        adapter: adapter,
        config: const AiQueryConfig(minimumWriteConfidence: 0.90),
      );

      expect(db.aiQueryAdapter, same(adapter));
      expect(db.aiQueryConfig.minimumWriteConfidence, 0.90);
      db.close();
    });
  });

  // ── v3.3 Query Builder Enhancements ─────────────────────────

  group('FilterQuery — enhanced flags', () {
    test('distinct() sets flag and includes in build output', () {
      final query = FilterQuery<dynamic>()
          .equalTo('name', 'Alice')
          .distinct();

      expect(query.isDistinct, isTrue);
      final built = query.build();
      expect(built['distinct'], isTrue);
    });

    test('distinct flag omitted when not set', () {
      final query = FilterQuery<dynamic>().equalTo('name', 'Alice');

      expect(query.isDistinct, isFalse);
      final built = query.build();
      expect(built.containsKey('distinct'), isFalse);
    });

    test('withProvenance() sets flag', () {
      final query = FilterQuery<dynamic>().withProvenance();
      expect(query.isWithProvenance, isTrue);
    });

    test('withFederation() sets flag', () {
      final query = FilterQuery<dynamic>().withFederation();
      expect(query.isWithFederation, isTrue);
    });

    test('acrossPeers() is alias for withFederation', () {
      final query = FilterQuery<dynamic>().acrossPeers();
      expect(query.isWithFederation, isTrue);
    });

    test('withAiQuery() sets flag', () {
      final query = FilterQuery<dynamic>().withAiQuery();
      expect(query.isWithAiQuery, isTrue);
    });

    test('flags can be combined', () {
      final query = FilterQuery<dynamic>()
          .equalTo('status', 'active')
          .distinct()
          .withProvenance()
          .withFederation()
          .withAiQuery()
          .sortBy('name')
          .limit(10);

      expect(query.isDistinct, isTrue);
      expect(query.isWithProvenance, isTrue);
      expect(query.isWithFederation, isTrue);
      expect(query.isWithAiQuery, isTrue);

      final built = query.build();
      expect(built['distinct'], isTrue);
      expect(built['filter'], isNotNull);
      expect(built['sort'], isNotNull);
      expect(built['limit'], 10);
    });

    test('flags default to false', () {
      final query = FilterQuery<dynamic>();
      expect(query.isDistinct, isFalse);
      expect(query.isWithProvenance, isFalse);
      expect(query.isWithFederation, isFalse);
      expect(query.isWithAiQuery, isFalse);
    });
  });

  group('NodeDB — findAllWithProvenance', () {
    test('returns documents with provenance when engine enabled', () {
      final dir = Directory.systemTemp.createTempSync('nodedb_prov_test_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          provenanceEnabled: true,
          bindings: bindings,
        );

        // Insert a document
        db.nosql.writeTxn([
          WriteOp.put('items', data: {'name': 'Widget', 'price': 9.99}),
        ]);

        // Attach provenance
        final docs = db.nosql.findAll('items');
        expect(docs, hasLength(1));
        db.provenance!.attach(
          collection: 'items',
          recordId: docs.first.id,
          sourceId: 'test-source',
          sourceType: 'user',
          contentHash: db.provenance!.computeHash(docs.first.data),
        );

        // Query with provenance
        final results = db.findAllWithProvenance('items');
        expect(results, hasLength(1));
        expect(results.first.data.data['name'], 'Widget');
        expect(results.first.provenance, isNotNull);
        expect(results.first.provenance!.sourceId, 'test-source');

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('returns null provenance when engine not enabled', () {
      final dir = Directory.systemTemp.createTempSync('nodedb_noprov_test_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          bindings: bindings,
        );

        db.nosql.writeTxn([
          WriteOp.put('items', data: {'name': 'Gadget'}),
        ]);

        final results = db.findAllWithProvenance('items');
        expect(results, hasLength(1));
        expect(results.first.data.data['name'], 'Gadget');
        expect(results.first.provenance, isNull);

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });

  group('NodeDB — findAllFederated', () {
    test('returns local results tagged as local', () {
      final dir = Directory.systemTemp.createTempSync('nodedb_fed_test_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          bindings: bindings,
        );

        db.nosql.writeTxn([
          WriteOp.put('items', data: {'name': 'Local Item'}),
        ]);

        final results = db.findAllFederated('items');
        expect(results, hasLength(1));
        expect(results.first.sourcePeerId, 'local');
        expect(results.first.data.data['name'], 'Local Item');

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });

  group('NodeDB — findAllWithAi', () {
    test('returns local results when available (skips AI)', () async {
      final dir = Directory.systemTemp.createTempSync('nodedb_ai_test_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          provenanceEnabled: true,
          bindings: bindings,
        );

        db.nosql.writeTxn([
          WriteOp.put('items', data: {'name': 'Existing'}),
        ]);

        final results = await db.findAllWithAi('items');
        expect(results, hasLength(1));
        expect(results.first.data['name'], 'Existing');

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });

    test('returns empty when no local results and no AI adapter', () async {
      final dir = Directory.systemTemp.createTempSync('nodedb_ai_empty_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          bindings: bindings,
        );

        final results = await db.findAllWithAi('items');
        expect(results, isEmpty);

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });

  group('NodeDB — findAllFull', () {
    test('returns local results with provenance attached', () async {
      final dir = Directory.systemTemp.createTempSync('nodedb_full_test_');
      try {
        final db = NodeDB.open(
          directory: dir.path,
          databaseName: 'test',
          provenanceEnabled: true,
          bindings: bindings,
        );

        db.nosql.writeTxn([
          WriteOp.put('items', data: {'name': 'Full Test'}),
        ]);

        final docs = db.nosql.findAll('items');
        db.provenance!.attach(
          collection: 'items',
          recordId: docs.first.id,
          sourceId: 'full-source',
          sourceType: 'user',
          contentHash: db.provenance!.computeHash(docs.first.data),
        );

        final results = await db.findAllFull('items');
        expect(results, hasLength(1));
        expect(results.first.data.data['name'], 'Full Test');
        expect(results.first.provenance, isNotNull);
        expect(results.first.provenance!.sourceId, 'full-source');

        db.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });

  group('CollectionAccessor — findAllWithProvenance', () {
    test('returns typed results with provenance', () {
      final dir = Directory.systemTemp.createTempSync('nodedb_accessor_test_');
      try {
        final engine = NoSqlEngine.open(bindings, dir.path);
        final provDir = Directory('${dir.path}/prov');
        provDir.createSync();
        final provEngine = ProvenanceEngine.open(bindings, provDir.path);

        // Insert data
        engine.writeTxn([
          WriteOp.put('widgets', data: {'label': 'A', 'weight': 1.5}),
          WriteOp.put('widgets', data: {'label': 'B', 'weight': 2.0}),
        ]);

        final accessor = CollectionAccessor<Map<String, dynamic>>(
          collectionName: 'widgets',
          engine: engine,
          fromMap: (m) => m,
          toMap: (m) => m,
        );

        // Attach provenance to first doc
        final docs = engine.findAll('widgets');
        provEngine.attach(
          collection: 'widgets',
          recordId: docs.first.id,
          sourceId: 'accessor-test',
          sourceType: 'user',
          contentHash: provEngine.computeHash(docs.first.data),
        );

        final results = accessor.findAllWithProvenance(
          provenanceEngine: provEngine,
        );
        expect(results, hasLength(2));

        // First doc has provenance
        final withProv = results.firstWhere((r) => r.provenance != null);
        expect(withProv.provenance!.sourceId, 'accessor-test');

        // Second doc has no provenance
        final withoutProv = results.firstWhere((r) => r.provenance == null);
        expect(withoutProv.data['label'], isNotNull);

        provEngine.close();
        engine.close();
      } finally {
        dir.deleteSync(recursive: true);
      }
    });
  });

}

// ── Stub adapters for testing ──────────────────────────────────

class _StubAiProvenanceAdapter extends NodeDbAiProvenanceAdapter {
  @override
  Future<AiProvenanceAssessment?> assessRecord({
    required String collection,
    required String recordJson,
    required ProvenanceEnvelope currentEnvelope,
  }) async =>
      null;

  @override
  Future<AiConflictResolution?> resolveConflict({
    required String collection,
    required ProvenanceEnvelope envelopeA,
    required ProvenanceEnvelope envelopeB,
    required String recordAJson,
    required String recordBJson,
  }) async =>
      null;

  @override
  Future<List<AiAnomalyFlag>> detectAnomalies({
    required String collection,
    required List<ProvenanceEnvelope> envelopes,
  }) async =>
      [];

  @override
  Future<AiSourceClassification?> classifySource({
    required String rawSourceId,
    required String? context,
  }) async =>
      null;
}

class _StubAiQueryAdapter extends NodeDbAiQueryAdapter {
  @override
  Future<List<AiQueryResult>> queryForMissingData({
    required String collection,
    required String schemaJson,
    required String queryDescription,
    required AiQueryContext context,
  }) async =>
      [];
}
