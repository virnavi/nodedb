import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';
import 'package:test/test.dart';

void main() {
  late NodeDbBindings bindings;
  late Directory tempDir;

  setUpAll(() {
    bindings = NodeDbBindings(loadNodeDbLibrary());
  });

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('nodedb_inspector_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('NoSqlPanel', () {
    late NoSqlEngine engine;
    late NoSqlPanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = NoSqlPanel(engine);
    });

    tearDown(() => engine.close());

    test('collectionStats returns counts', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice'}),
        WriteOp.put('users', data: {'name': 'Bob'}),
        WriteOp.put('posts', data: {'title': 'Hello'}),
      ]);

      final stats = panel.collectionStats();
      expect(stats.length, 2);
      // Values should sum to 3
      expect(stats.values.fold(0, (int s, c) => s + c), 3);
    });

    test('documentPreview returns limited docs', () {
      for (var i = 0; i < 5; i++) {
        engine.writeTxn([WriteOp.put('items', data: {'n': i})]);
      }

      final preview = panel.documentPreview('items', limit: 3);
      expect(preview, hasLength(3));
    });

    test('documentDetail returns single doc', () {
      engine.writeTxn([WriteOp.put('users', data: {'name': 'Test'})]);
      final docs = engine.findAll('users');
      final doc = panel.documentDetail('users', docs.first.id);
      expect(doc, isNotNull);
      expect(doc!.data['name'], 'Test');
    });

    test('summary includes totalDocuments', () {
      engine.writeTxn([
        WriteOp.put('a', data: {'x': 1}),
        WriteOp.put('b', data: {'x': 2}),
      ]);

      final summary = panel.summary();
      expect(summary['totalDocuments'], 2);
      expect(summary['collections'], isA<Map>());
      expect(summary['schemaFingerprint'], isA<String>());
    });
  });

  group('GraphPanel', () {
    late GraphEngine engine;
    late GraphPanel panel;

    setUp(() {
      final dir = Directory('${tempDir.path}/graph')..createSync();
      engine = GraphEngine.open(bindings, dir.path);
      panel = GraphPanel(engine);
    });

    tearDown(() => engine.close());

    test('stats returns nodeCount', () {
      engine.addNode('person', {'name': 'A'});
      engine.addNode('person', {'name': 'B'});

      final stats = panel.stats();
      expect(stats['nodeCount'], 2);
    });

    test('nodePreview returns limited nodes', () {
      for (var i = 0; i < 5; i++) {
        engine.addNode('person', {'n': i});
      }

      final preview = panel.nodePreview(limit: 3);
      expect(preview, hasLength(3));
    });

    test('nodeDetail includes edges', () {
      final a = engine.addNode('person', {'name': 'A'});
      final b = engine.addNode('person', {'name': 'B'});
      engine.addEdge('knows', a.id, b.id);

      final detail = panel.nodeDetail(a.id);
      expect(detail, isNotNull);
      expect(detail!['node'], isA<GraphNode>());
      expect((detail['edgesFrom'] as List).length, 1);
    });

    test('traversal runs BFS', () {
      final a = engine.addNode('person', {'name': 'A'});
      final b = engine.addNode('person', {'name': 'B'});
      engine.addEdge('knows', a.id, b.id);

      final result = panel.traversal(a.id, 'bfs');
      expect(result['nodes'], contains(a.id));
      expect(result['nodes'], contains(b.id));
    });

    test('summary', () {
      engine.addNode('person', {'name': 'A'});
      final summary = panel.summary();
      expect(summary['nodeCount'], 1);
    });
  });

  group('FederationPanel', () {
    late FederationEngine engine;
    late FederationPanel panel;

    setUp(() {
      final dir = Directory('${tempDir.path}/fed')..createSync();
      engine = FederationEngine.open(bindings, dir.path);
      panel = FederationPanel(engine);
    });

    tearDown(() => engine.close());

    test('peerList returns all peers', () {
      engine.addPeer('p1', 'ws://p1:8080');
      engine.addPeer('p2', 'ws://p2:8080');
      expect(panel.peerList(), hasLength(2));
    });

    test('peerDetail includes group memberships', () {
      final peer = engine.addPeer('p1', 'ws://p1:8080');
      final group = engine.addGroup('team');
      engine.addMember(group.id, peer.id);

      final detail = panel.peerDetail(peer.id);
      expect(detail, isNotNull);
      expect(detail!['peer'], isA<NodePeer>());
      expect((detail['groupIds'] as List).length, 1);
    });

    test('topology returns full graph', () {
      final peer = engine.addPeer('p1', 'ws://p1:8080');
      final group = engine.addGroup('team');
      engine.addMember(group.id, peer.id);

      final topo = panel.topology();
      expect((topo['peers'] as List).length, 1);
      expect((topo['groups'] as List).length, 1);
      expect((topo['memberships'] as List).length, 1);
    });

    test('summary', () {
      engine.addPeer('p1', 'ws://p1:8080');
      engine.addGroup('g1');
      final summary = panel.summary();
      expect(summary['peerCount'], 1);
      expect(summary['groupCount'], 1);
    });
  });

  group('DacPanel', () {
    late DacEngine engine;
    late DacPanel panel;

    setUp(() {
      final dir = Directory('${tempDir.path}/dac')..createSync();
      engine = DacEngine.open(bindings, dir.path);
      panel = DacPanel(engine);
    });

    tearDown(() => engine.close());

    test('ruleList returns all rules', () {
      engine.addRule(
        collection: 'users', subjectType: 'peer',
        subjectId: 'p1', permission: 'allow',
      );
      engine.addRule(
        collection: 'posts', subjectType: 'peer',
        subjectId: 'p1', permission: 'deny',
      );
      expect(panel.ruleList(), hasLength(2));
    });

    test('ruleList filters by collection', () {
      engine.addRule(
        collection: 'users', subjectType: 'peer',
        subjectId: 'p1', permission: 'allow',
      );
      engine.addRule(
        collection: 'posts', subjectType: 'peer',
        subjectId: 'p1', permission: 'deny',
      );
      expect(panel.ruleList(collection: 'users'), hasLength(1));
    });

    test('ruleStats breaks down by permission', () {
      engine.addRule(
        collection: 'a', subjectType: 'peer',
        subjectId: 'p1', permission: 'allow',
      );
      engine.addRule(
        collection: 'b', subjectType: 'peer',
        subjectId: 'p1', permission: 'allow',
      );
      engine.addRule(
        collection: 'c', subjectType: 'peer',
        subjectId: 'p1', permission: 'deny',
      );

      final stats = panel.ruleStats();
      expect(stats['allow'], 2);
      expect(stats['deny'], 1);
    });

    test('summary', () {
      engine.addRule(
        collection: 'users', subjectType: 'peer',
        subjectId: 'p1', permission: 'allow',
      );
      final summary = panel.summary();
      expect(summary['ruleCount'], 1);
    });
  });

  group('ProvenancePanel', () {
    late ProvenanceEngine engine;
    late ProvenancePanel panel;

    setUp(() {
      final dir = Directory('${tempDir.path}/prov')..createSync();
      engine = ProvenanceEngine.open(bindings, dir.path);
      panel = ProvenancePanel(engine);
    });

    tearDown(() => engine.close());

    test('stats returns breakdowns', () {
      engine.attach(
        collection: 'users', recordId: 1,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );
      engine.attach(
        collection: 'users', recordId: 2,
        sourceId: 's2', sourceType: 'peer', contentHash: 'h2',
      );

      final stats = panel.stats();
      expect(stats['totalCount'], 2);
      expect(stats['sourceTypeBreakdown'], isA<Map>());
      expect(stats['verificationBreakdown'], isA<Map>());
    });

    test('confidenceHistogram buckets correctly', () {
      engine.attach(
        collection: 'items', recordId: 1,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );

      final histogram = panel.confidenceHistogram('items');
      expect(histogram.length, 5);
      // Default confidence should be in one of the buckets
      final total = histogram.values.fold(0, (int s, c) => s + c);
      expect(total, 1);
    });

    test('envelopesForRecord returns matching envelopes', () {
      engine.attach(
        collection: 'users', recordId: 42,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );
      engine.attach(
        collection: 'users', recordId: 42,
        sourceId: 's2', sourceType: 'peer', contentHash: 'h2',
      );

      final envs = panel.envelopesForRecord('users', 42);
      expect(envs.length, greaterThanOrEqualTo(2));
    });

    test('summary', () {
      engine.attach(
        collection: 'items', recordId: 1,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );
      final summary = panel.summary();
      expect(summary['envelopeCount'], 1);
    });
  });

  group('KeyResolverPanel', () {
    late KeyResolverEngine engine;
    late KeyResolverPanel panel;

    setUp(() {
      final dir = Directory('${tempDir.path}/kr')..createSync();
      engine = KeyResolverEngine.open(bindings, dir.path);
      panel = KeyResolverPanel(engine);
    });

    tearDown(() => engine.close());

    test('keyList returns all keys', () {
      engine.supplyKey(
        pkiId: 'pki-1', userId: 'user-1',
        publicKeyHex: 'aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd',
      );
      expect(panel.keyList(), hasLength(1));
    });

    test('keyStats includes trust level breakdown', () {
      engine.supplyKey(
        pkiId: 'pki-1', userId: 'user-1',
        publicKeyHex: 'aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd',
      );
      final stats = panel.keyStats();
      expect(stats['totalKeys'], 1);
      expect(stats['trustLevelBreakdown'], isA<Map>());
      expect(stats['trustAllActive'], isFalse);
    });

    test('summary', () {
      final summary = panel.summary();
      expect(summary['keyCount'], 0);
      expect(summary['trustAllActive'], isFalse);
    });
  });

  group('SchemaPanel', () {
    late NoSqlEngine engine;
    late SchemaPanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = SchemaPanel(engine, null);
    });

    tearDown(() => engine.close());

    test('overview returns schemas and fingerprint', () {
      final overview = panel.overview();
      expect(overview['schemas'], isA<List>());
      expect(overview['collections'], isA<List>());
      expect(overview['fingerprint'], isA<String>());
    });

    test('collectionDetail returns document count', () {
      engine.writeTxn([
        WriteOp.put('users', data: {'name': 'A'}),
        WriteOp.put('users', data: {'name': 'B'}),
      ]);

      final detail = panel.collectionDetail('users');
      expect(detail['name'], 'users');
      expect(detail['documentCount'], 2);
    });
  });

  group('TriggerPanel', () {
    late NoSqlEngine engine;
    late TriggerPanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = TriggerPanel(engine);
    });

    tearDown(() => engine.close());

    test('listTriggers returns empty for fresh db', () {
      expect(panel.listTriggers(), isEmpty);
    });

    test('triggerCount reflects registered triggers', () {
      engine.registerTrigger(
        collection: 'users', event: 'insert', timing: 'after',
      );
      engine.registerTrigger(
        collection: 'orders', event: 'delete', timing: 'before',
      );
      expect(panel.triggerCount(), 2);
    });

    test('triggersByCollection groups correctly', () {
      engine.registerTrigger(
        collection: 'users', event: 'insert', timing: 'after',
      );
      engine.registerTrigger(
        collection: 'users', event: 'update', timing: 'before',
      );
      engine.registerTrigger(
        collection: 'orders', event: 'insert', timing: 'after',
      );

      final grouped = panel.triggersByCollection();
      expect(grouped.keys.length, 2);
    });

    test('enabledTriggers and disabledTriggers', () {
      final id = engine.registerTrigger(
        collection: 'users', event: 'insert', timing: 'after',
      );
      engine.registerTrigger(
        collection: 'orders', event: 'insert', timing: 'after',
      );
      engine.setTriggerEnabled(id, enabled: false);

      expect(panel.enabledTriggers().length, 1);
      expect(panel.disabledTriggers().length, 1);
    });

    test('summary includes counts', () {
      engine.registerTrigger(
        collection: 'users', event: 'insert', timing: 'after',
      );
      final summary = panel.summary();
      expect(summary['totalTriggers'], 1);
      expect(summary['enabled'], 1);
      expect(summary['disabled'], 0);
      expect(summary['triggers'], isA<List>());
    });
  });

  group('SingletonPanel', () {
    late NoSqlEngine engine;
    late SingletonPanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = SingletonPanel(engine);
    });

    tearDown(() => engine.close());

    test('singletonNames returns empty for fresh db', () {
      expect(panel.singletonNames(), isEmpty);
    });

    test('singletonNames detects singleton collections', () {
      engine.singletonCreate('app_config', {'theme': 'light'});
      engine.writeTxn([WriteOp.put('users', data: {'name': 'Alice'})]);

      final names = panel.singletonNames();
      expect(names.length, 1);
      expect(names.first, contains('app_config'));
    });

    test('singletonData returns current data', () {
      engine.singletonCreate('settings', {'volume': 50});
      engine.singletonPut('settings', {'volume': 75});

      final doc = panel.singletonData('settings');
      expect(doc.id, 1);
      expect(doc.data['volume'], 75);
    });

    test('singletonPreview returns all singletons with data', () {
      engine.singletonCreate('config_a', {'key': 'a'});
      engine.singletonCreate('config_b', {'key': 'b'});

      final preview = panel.singletonPreview();
      expect(preview.length, 2);
      expect(preview.every((p) => p.containsKey('data')), isTrue);
    });

    test('summary includes count and names', () {
      engine.singletonCreate('app_state', {'init': true});
      final summary = panel.summary();
      expect(summary['count'], 1);
      expect(summary['collections'], isA<List>());
    });
  });

  group('PreferencePanel', () {
    late NoSqlEngine engine;
    late PreferencePanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = PreferencePanel(engine);
    });

    tearDown(() => engine.close());

    test('keys returns empty for fresh store', () {
      expect(panel.keys('prefs'), isEmpty);
    });

    test('keys returns stored keys', () {
      engine.prefSet('prefs', 'locale', 'en');
      engine.prefSet('prefs', 'theme', 'dark');

      expect(panel.keys('prefs'), unorderedEquals(['locale', 'theme']));
    });

    test('getValue returns preference value', () {
      engine.prefSet('prefs', 'locale', 'fr');
      final resp = panel.getValue('prefs', 'locale');
      expect(resp['found'], isTrue);
      expect(resp['value'], 'fr');
    });

    test('allValues returns all key-value pairs', () {
      engine.prefSet('prefs', 'locale', 'en');
      engine.prefSet('prefs', 'font_size', 14);

      final all = panel.allValues('prefs');
      expect(all['locale'], 'en');
      expect(all['font_size'], 14);
    });

    test('storeSummary includes key count', () {
      engine.prefSet('prefs', 'a', 1);
      engine.prefSet('prefs', 'b', 2);

      final summary = panel.storeSummary('prefs');
      expect(summary['store'], 'prefs');
      expect(summary['keyCount'], 2);
      expect(summary['keys'], hasLength(2));
    });
  });

  group('AccessHistoryPanel', () {
    late NoSqlEngine engine;
    late AccessHistoryPanel panel;

    setUp(() {
      engine = NoSqlEngine.open(bindings, tempDir.path);
      panel = AccessHistoryPanel(engine);
    });

    tearDown(() => engine.close());

    test('count returns zero for fresh db', () {
      expect(panel.count(), 0);
    });

    test('count reflects access history entries', () {
      // Trigger access history by reading
      engine.writeTxn([WriteOp.put('items', data: {'n': 1})]);
      engine.findAll('items');

      expect(panel.count(), greaterThan(0));
    });

    test('query returns entries', () {
      engine.writeTxn([WriteOp.put('items', data: {'n': 1})]);
      engine.findAll('items');

      final entries = panel.query();
      expect(entries, isNotEmpty);
    });

    test('heatmap groups by collection', () {
      engine.writeTxn([
        WriteOp.put('items', data: {'n': 1}),
        WriteOp.put('orders', data: {'n': 2}),
      ]);
      engine.findAll('items');
      engine.findAll('orders');

      final heatmap = panel.heatmap();
      expect(heatmap, isA<Map<String, int>>());
    });

    test('summary includes totalEntries', () {
      final summary = panel.summary();
      expect(summary['totalEntries'], isA<int>());
      expect(summary['heatmap'], isA<Map>());
    });
  });

  group('AiPanel', () {
    late NodeDB db;
    late AiPanel panel;

    setUp(() {
      db = NodeDB.open(
        directory: '${tempDir.path}/ai_panel',
        bindings: bindings,
        databaseName: 'test',
        provenanceEnabled: true,
      );
      panel = AiPanel(db.provenance!, db.aiProvenance, db.aiQuery);
    });

    tearDown(() => db.close());

    test('stats returns AI counts', () {
      // Attach some envelopes to provenance
      db.provenance!.attach(
        collection: 'items', recordId: 1,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );
      db.provenance!.attach(
        collection: 'items', recordId: 2,
        sourceId: 's2', sourceType: 'peer', contentHash: 'h2',
      );

      final stats = panel.stats();
      expect(stats['totalEnvelopes'], greaterThanOrEqualTo(2));
      expect(stats['aiAugmented'], isA<int>());
      expect(stats['aiOriginated'], isA<int>());
      expect(stats['anomalyFlagged'], isA<int>());
      expect(stats['anomalySeverity'], isA<Map>());
    });

    test('aiProvenanceConfig returns config', () {
      final config = panel.aiProvenanceConfig();
      expect(config['enabled'], isTrue);
    });

    test('aiQueryConfig returns config', () {
      final config = panel.aiQueryConfig();
      expect(config['enabled'], isTrue);
    });

    test('summary includes AI flags', () {
      final summary = panel.summary();
      expect(summary['aiAugmented'], isA<int>());
      expect(summary['aiOriginated'], isA<int>());
      expect(summary['anomalyFlagged'], isA<int>());
      expect(summary['aiProvenanceEnabled'], isTrue);
      expect(summary['aiQueryEnabled'], isTrue);
    });
  });

  group('JSON Serializers', () {
    late NodeDbBindings b;
    late Directory dir;

    setUp(() {
      b = bindings;
      dir = tempDir;
    });

    test('documentToJson serializes Document', () {
      final engine = NoSqlEngine.open(b, dir.path);
      engine.writeTxn([WriteOp.put('users', data: {'name': 'Alice'})]);
      final doc = engine.findAll('users').first;

      final json = documentToJson(doc);
      expect(json['id'], doc.id);
      expect(json['collection'], doc.collection);
      expect(json['data']['name'], 'Alice');
      expect(json['createdAt'], isA<String>());
      expect(json['updatedAt'], isA<String>());

      engine.close();
    });

    test('graphNodeToJson serializes GraphNode', () {
      final d = Directory('${dir.path}/gj')..createSync();
      final engine = GraphEngine.open(b, d.path);
      final node = engine.addNode('person', {'name': 'Bob'});

      final json = graphNodeToJson(node);
      expect(json['id'], node.id);
      expect(json['label'], 'person');
      expect(json['data']['name'], 'Bob');

      engine.close();
    });

    test('graphEdgeToJson serializes GraphEdge', () {
      final d = Directory('${dir.path}/ge')..createSync();
      final engine = GraphEngine.open(b, d.path);
      final a = engine.addNode('person', {'name': 'A'});
      final bNode = engine.addNode('person', {'name': 'B'});
      final edge = engine.addEdge('knows', a.id, bNode.id);

      final json = graphEdgeToJson(edge);
      expect(json['id'], edge.id);
      expect(json['label'], 'knows');
      expect(json['source'], a.id);
      expect(json['target'], bNode.id);

      engine.close();
    });

    test('nodePeerToJson serializes NodePeer', () {
      final d = Directory('${dir.path}/fp')..createSync();
      final engine = FederationEngine.open(b, d.path);
      final peer = engine.addPeer('alice', 'ws://alice:8080');

      final json = nodePeerToJson(peer);
      expect(json['id'], peer.id);
      expect(json['name'], 'alice');
      expect(json['endpoint'], 'ws://alice:8080');

      engine.close();
    });

    test('nodeGroupToJson serializes NodeGroup', () {
      final d = Directory('${dir.path}/fg')..createSync();
      final engine = FederationEngine.open(b, d.path);
      final group = engine.addGroup('team');

      final json = nodeGroupToJson(group);
      expect(json['id'], group.id);
      expect(json['name'], 'team');

      engine.close();
    });

    test('accessRuleToJson serializes AccessRule', () {
      final d = Directory('${dir.path}/dac')..createSync();
      final engine = DacEngine.open(b, d.path);
      final rule = engine.addRule(
        collection: 'users',
        subjectType: 'peer',
        subjectId: 'p1',
        permission: 'allow',
      );

      final json = accessRuleToJson(rule);
      expect(json['id'], rule.id);
      expect(json['collection'], 'users');
      expect(json['subjectType'], 'peer');
      expect(json['permission'], 'allow');

      engine.close();
    });

    test('provenanceEnvelopeToJson serializes ProvenanceEnvelope', () {
      final d = Directory('${dir.path}/prov')..createSync();
      final engine = ProvenanceEngine.open(b, d.path);
      engine.attach(
        collection: 'items', recordId: 1,
        sourceId: 's1', sourceType: 'user', contentHash: 'h1',
      );
      final envs = engine.getForRecord('items', 1);
      expect(envs, isNotEmpty);

      final json = provenanceEnvelopeToJson(envs.first);
      expect(json['id'], envs.first.id);
      expect(json['collection'], 'items');
      expect(json['recordId'], 1);
      expect(json['sourceId'], 's1');
      expect(json['createdAtUtc'], isA<String>());

      engine.close();
    });

    test('keyEntryToJson serializes KeyEntry', () {
      final d = Directory('${dir.path}/kr')..createSync();
      final engine = KeyResolverEngine.open(b, d.path);
      engine.supplyKey(
        pkiId: 'pki-1', userId: 'user-1',
        publicKeyHex: 'aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd',
      );
      final keys = engine.allKeys();
      expect(keys, isNotEmpty);

      final json = keyEntryToJson(keys.first);
      expect(json['id'], keys.first.id);
      expect(json['pkiId'], 'pki-1');
      expect(json['userId'], 'user-1');
      expect(json['createdAtUtc'], isA<String>());

      engine.close();
    });
  });

  group('CommandRouter', () {
    late NodeDB db;
    late NodeDbInspector inspector;
    late CommandRouter router;

    setUp(() {
      db = NodeDB.open(
        directory: '${tempDir.path}/cmd',
        bindings: bindings,
        databaseName: 'test',
        graphEnabled: true,
        dacEnabled: true,
        provenanceEnabled: true,
        keyResolverEnabled: true,
      );
      inspector = NodeDbInspector(db);
      router = CommandRouter(inspector);
    });

    tearDown(() => db.close());

    test('snapshot returns map with version', () {
      final result = router.dispatch('snapshot', {});
      expect(result, isA<Map>());
      expect(result['version'], isA<int>());
      expect(result['nosql'], isA<Map>());
    });

    test('enabledPanels returns list of strings', () {
      final result = router.dispatch('enabledPanels', {});
      expect(result, isA<List>());
      expect(result, contains('nosql'));
      expect(result, contains('graph'));
      expect(result, contains('federation'));
    });

    test('panel dispatch to nosql collectionNames', () {
      db.nosql.writeTxn([
        WriteOp.put('users', data: {'name': 'A'}),
      ]);

      final result = router.dispatch('panel', {
        'panel': 'nosql',
        'action': 'collectionNames',
      });
      expect(result, isA<List>());
    });

    test('panel dispatch to graph stats', () {
      final result = router.dispatch('panel', {
        'panel': 'graph',
        'action': 'stats',
      });
      expect(result, isA<Map>());
      expect(result['nodeCount'], isA<int>());
    });

    test('panel dispatch to federation peerList', () {
      final result = router.dispatch('panel', {
        'panel': 'federation',
        'action': 'peerList',
      });
      expect(result, isA<List>());
    });

    test('unknown command throws', () {
      expect(
        () => router.dispatch('nonexistent', {}),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('unknown panel throws', () {
      expect(
        () => router.dispatch('panel', {'panel': 'fake', 'action': 'x'}),
        throwsA(isA<ArgumentError>()),
      );
    });

    test('missing panel/action throws', () {
      expect(
        () => router.dispatch('panel', {}),
        throwsA(isA<ArgumentError>()),
      );
    });
  });

  group('NodeDbInspector', () {
    test('snapshot with nosql-only', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        bindings: bindings,
        databaseName: 'test',
      );

      final inspector = NodeDbInspector(db);
      final snap = inspector.snapshot();

      expect(snap['version'], isA<int>());
      expect(snap['nosql'], isA<Map>());
      expect(snap['triggers'], isA<Map>());
      expect(snap['singletons'], isA<Map>());
      expect(snap['federation'], isA<Map>());
      expect(snap['accessHistory'], isA<Map>());
      expect(snap.containsKey('graph'), isFalse);

      db.close();
    });

    test('enabledPanels lists active panels', () {
      final db = NodeDB.open(
        directory: tempDir.path,
        bindings: bindings,
        databaseName: 'test',
      );

      final inspector = NodeDbInspector(db);
      final panels = inspector.enabledPanels();

      expect(panels, contains('nosql'));
      expect(panels, contains('schema'));
      expect(panels, contains('triggers'));
      expect(panels, contains('singletons'));
      expect(panels, contains('preferences'));
      expect(panels, contains('federation'));
      expect(panels, contains('accessHistory'));
      expect(panels, isNot(contains('graph')));

      db.close();
    });

    test('snapshot with multiple engines', () {
      final db = NodeDB.open(
        directory: '${tempDir.path}/multi',
        bindings: bindings,
        databaseName: 'test',
        graphEnabled: true,
        dacEnabled: true,
        provenanceEnabled: true,
        keyResolverEnabled: true,
      );

      final inspector = NodeDbInspector(db);
      final panels = inspector.enabledPanels();
      expect(panels, containsAll([
        'nosql', 'schema', 'graph', 'federation',
        'dac', 'provenance', 'keyResolver', 'accessHistory', 'ai',
      ]));

      final snap = inspector.snapshot();
      expect(snap['version'], greaterThan(0));
      expect(snap['nosql'], isA<Map>());
      expect(snap['graph'], isA<Map>());
      expect(snap['federation'], isA<Map>());
      expect(snap['dac'], isA<Map>());
      expect(snap['provenance'], isA<Map>());
      expect(snap['keyResolver'], isA<Map>());
      expect(snap['accessHistory'], isA<Map>());
      expect(snap['ai'], isA<Map>());

      db.close();
    });
  });

  group('InspectorServer', () {
    late NodeDB db;
    late NodeDbInspector inspector;

    setUp(() {
      db = NodeDB.open(
        directory: '${tempDir.path}/srv',
        bindings: bindings,
        databaseName: 'test',
      );
      inspector = NodeDbInspector(db);
    });

    tearDown(() async {
      await inspector.stop();
      db.close();
    });

    test('start and stop lifecycle', () async {
      expect(inspector.isRunning, isFalse);
      await inspector.start();
      expect(inspector.isRunning, isTrue);
      await inspector.stop();
      expect(inspector.isRunning, isFalse);
    });

    test('start is idempotent', () async {
      await inspector.start();
      await inspector.start(); // second call should be no-op
      expect(inspector.isRunning, isTrue);
    });

    test('GET / returns HTML', () async {
      final server = InspectorServer(inspector, port: 0);
      await server.start();

      final client = HttpClient();
      final request = await client.get(
        'localhost', server.port, '/',
      );
      final response = await request.close();

      expect(response.statusCode, 200);
      expect(response.headers.contentType?.mimeType, 'text/html');
      final body = await response.transform(utf8.decoder).join();
      expect(body, contains('NodeDB Inspector'));

      client.close();
      await server.stop();
    });

    test('GET /other returns 404', () async {
      final server = InspectorServer(inspector, port: 0);
      await server.start();

      final client = HttpClient();
      final request = await client.get(
        'localhost', server.port, '/nonexistent',
      );
      final response = await request.close();

      expect(response.statusCode, 404);

      client.close();
      await server.stop();
    });

    test('WebSocket connects without passcode', () async {
      final server = InspectorServer(inspector, port: 0);
      await server.start();

      final ws = await WebSocket.connect('ws://localhost:${server.port}/ws');

      // Should receive a snapshot push shortly
      final completer = Completer<Map<String, dynamic>>();
      ws.listen((data) {
        if (!completer.isCompleted) {
          completer.complete(jsonDecode(data as String));
        }
      });

      final msg = await completer.future.timeout(const Duration(seconds: 10));
      expect(msg['cmd'], 'snapshot');
      expect(msg['data'], isA<Map>());

      await ws.close();
      await server.stop();
    });

    test('WebSocket with passcode auth succeeds', () async {
      final server = InspectorServer(
        inspector,
        port: 0,
        passcode: 'secret123',
      );
      await server.start();

      final ws = await WebSocket.connect('ws://localhost:${server.port}/ws');

      // Send auth
      ws.add(jsonEncode({'auth': 'secret123'}));

      final messages = <Map<String, dynamic>>[];
      final completer = Completer<void>();
      ws.listen((data) {
        messages.add(jsonDecode(data as String));
        // Expect ok response + snapshot
        if (messages.length >= 2 && !completer.isCompleted) {
          completer.complete();
        }
      });

      await completer.future.timeout(const Duration(seconds: 10));
      expect(messages[0]['ok'], isTrue);
      expect(messages[1]['cmd'], 'snapshot');

      await ws.close();
      await server.stop();
    });

    test('WebSocket with bad passcode disconnects', () async {
      final server = InspectorServer(
        inspector,
        port: 0,
        passcode: 'secret123',
      );
      await server.start();

      final ws = await WebSocket.connect('ws://localhost:${server.port}/ws');

      ws.add(jsonEncode({'auth': 'wrong'}));

      final completer = Completer<Map<String, dynamic>>();
      ws.listen(
        (data) {
          if (!completer.isCompleted) {
            completer.complete(jsonDecode(data as String));
          }
        },
      );

      final msg = await completer.future.timeout(const Duration(seconds: 10));
      expect(msg['ok'], isFalse);
      expect(msg['error'], 'bad_passcode');

      await ws.close();
      await server.stop();
    });

    test('WebSocket command dispatch', () async {
      // Add some data first
      db.nosql.writeTxn([
        WriteOp.put('users', data: {'name': 'Alice'}),
      ]);

      final server = InspectorServer(inspector, port: 0);
      await server.start();

      final ws = await WebSocket.connect('ws://localhost:${server.port}/ws');

      // Skip initial snapshot
      final initialSnap = Completer<void>();
      late StreamSubscription sub;
      sub = ws.listen((data) {
        if (!initialSnap.isCompleted) {
          initialSnap.complete();
          sub.cancel();
        }
      });
      await initialSnap.future.timeout(const Duration(seconds: 10));

      // Reconnect to send command
      final ws2 = await WebSocket.connect('ws://localhost:${server.port}/ws');

      // Skip snapshot, then send command
      final msgs = <Map<String, dynamic>>[];
      final gotResponse = Completer<void>();
      ws2.listen((data) {
        final msg = jsonDecode(data as String) as Map<String, dynamic>;
        msgs.add(msg);
        if (msg['cmd'] == 'enabledPanels' && !gotResponse.isCompleted) {
          gotResponse.complete();
        }
      });

      // Wait a moment for snapshot to arrive, then send command
      await Future.delayed(const Duration(milliseconds: 500));
      ws2.add(jsonEncode({'cmd': 'enabledPanels'}));

      await gotResponse.future.timeout(const Duration(seconds: 10));
      final cmdMsg = msgs.firstWhere((m) => m['cmd'] == 'enabledPanels');
      expect(cmdMsg['data'], isA<List>());
      expect(cmdMsg['data'], contains('nosql'));

      await ws2.close();
      await server.stop();
    });
  });

}
