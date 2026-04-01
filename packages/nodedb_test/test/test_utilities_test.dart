import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:nodedb_test/nodedb_test.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(() {
    // Ensure bindings are loaded once
    NodeDbBindings(loadNodeDbLibrary());
  });

  tearDown(() {
    TestNodeDB.cleanUp();
  });

  group('TestNodeDB', () {
    test('create returns working NodeDB instance', () {
      final db = TestNodeDB.create();
      expect(db.nosql.handle, isNonZero);
      db.writeTxn([WriteOp.put('test', data: {'x': 1})]);
      expect(db.count('test'), 1);
      db.close();
    });

    test('create with graph engine', () {
      final db = TestNodeDB.create(graph: true);
      expect(db.graph, isNotNull);
      db.close();
    });

    test('create with provenance', () {
      final db = TestNodeDB.create(provenance: true);
      expect(db.provenance, isNotNull);
      db.close();
    });

    test('tempDir creates unique directories', () {
      final dir1 = TestNodeDB.tempDir();
      final dir2 = TestNodeDB.tempDir();
      expect(dir1, isNot(dir2));
    });

    test('cleanUp removes temp directories', () {
      final dir = TestNodeDB.tempDir();
      expect(dir, isNotEmpty);
      TestNodeDB.cleanUp();
      // After cleanup, creating new dirs works
      final dir2 = TestNodeDB.tempDir();
      expect(dir2, isNotEmpty);
    });
  });

  group('TestSeed', () {
    late NodeDB db;
    late TestSeed seed;

    setUp(() {
      db = TestNodeDB.create();
      seed = TestSeed(db.nosql);
    });

    tearDown(() {
      db.close();
    });

    test('insert adds a single document', () {
      final doc = seed.insert('items', {'name': 'A'});
      expect(doc.data['name'], 'A');
      expect(db.count('items'), 1);
    });

    test('insertAll adds multiple documents', () {
      final docs = seed.insertAll('items', [
        {'name': 'A'},
        {'name': 'B'},
        {'name': 'C'},
      ]);
      expect(docs, hasLength(3));
      expect(db.count('items'), 3);
    });

    test('generate creates N documents with builder', () {
      final docs = seed.generate('nums', 5, (i) => {'val': i * 10});
      expect(docs, hasLength(5));
      expect(db.count('nums'), 5);
    });

    test('seedSampleData populates users and posts', () {
      seed.seedSampleData();
      expect(db.count('users'), 4);
      expect(db.count('posts'), 3);
    });
  });

  group('Matchers', () {
    late NodeDB db;

    setUp(() {
      db = TestNodeDB.create(graph: true);
    });

    tearDown(() {
      db.close();
    });

    test('hasDocumentData matches data fields', () {
      db.writeTxn([WriteOp.put('items', data: {'name': 'Test', 'val': 42})]);
      final doc = db.findAll('items').first;
      expect(doc, hasDocumentData({'name': 'Test'}));
      expect(doc, hasDocumentData({'val': 42}));
    });

    test('hasDocumentId matches document id', () {
      db.writeTxn([WriteOp.put('items', data: {'x': 1})]);
      final doc = db.findAll('items').first;
      expect(doc, hasDocumentId(doc.id));
    });

    test('isDocument matches id and data', () {
      db.writeTxn([WriteOp.put('items', data: {'name': 'X'})]);
      final doc = db.findAll('items').first;
      expect(doc, isDocument(id: doc.id, data: {'name': 'X'}));
    });

    test('hasNodeLabel matches graph node label', () {
      final node = db.graph!.addNode('person', {'name': 'A'});
      expect(node, hasNodeLabel('person'));
    });

    test('connectsNodes matches graph edge endpoints', () {
      final a = db.graph!.addNode('person', {'name': 'A'});
      final b = db.graph!.addNode('person', {'name': 'B'});
      final edge = db.graph!.addEdge('knows', a.id, b.id);
      expect(edge, connectsNodes(a.id, b.id));
    });
  });
}
