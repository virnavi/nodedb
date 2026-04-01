import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_ffi/nodedb_ffi.dart';
import 'package:test/test.dart';

void main() {
  late NodeDbBindings bindings;
  late Directory tempDir;

  setUpAll(() {
    bindings = NodeDbBindings(loadNodeDbLibrary());
  });

  setUp(() {
    tempDir = Directory.systemTemp.createTempSync('nodedb_graph_algo_test_');
  });

  tearDown(() {
    tempDir.deleteSync(recursive: true);
  });

  group('Graph — Edge update', () {
    test('updateEdge changes data', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final edge = graph.addEdge('link', a.id, b.id, data: {'color': 'red'});

      final updated = graph.updateEdge(edge.id, {'color': 'blue'});
      expect(updated.data['color'], 'blue');

      // Verify via getEdge
      final fetched = graph.getEdge(edge.id);
      expect(fetched, isNotNull);
      expect(fetched!.data['color'], 'blue');

      graph.close();
    });
  });

  group('Graph — Neighbors', () {
    test('neighbors returns connected node IDs', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', a.id, c.id);

      final result = graph.neighbors(a.id);
      expect(result, isNotEmpty);
      final neighborIds = result.map((n) => n.id).toSet();
      expect(neighborIds, contains(b.id));
      expect(neighborIds, contains(c.id));

      graph.close();
    });
  });

  group('Graph — Connected components', () {
    test('connectedComponents finds separate subgraphs', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      // Component 1: a - b
      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      graph.addEdge('link', a.id, b.id);

      // Component 2: c - d
      final c = graph.addNode('c', {});
      final d = graph.addNode('d', {});
      graph.addEdge('link', c.id, d.id);

      final components = graph.connectedComponents();
      expect(components.length, 2);

      graph.close();
    });

    test('single connected graph has one component', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', b.id, c.id);

      final components = graph.connectedComponents();
      expect(components.length, 1);
      expect(components[0].length, 3);

      graph.close();
    });
  });

  group('Graph — Cycle detection', () {
    test('hasCycle returns false for acyclic graph', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', b.id, c.id);

      expect(graph.hasCycle(), isFalse);

      graph.close();
    });

    test('hasCycle returns true for cyclic graph', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', b.id, c.id);
      graph.addEdge('link', c.id, a.id);

      expect(graph.hasCycle(), isTrue);

      graph.close();
    });

    test('findCycles returns the cycle', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', b.id, c.id);
      graph.addEdge('link', c.id, a.id);

      final cycles = graph.findCycles();
      expect(cycles, isNotEmpty);
      expect(cycles[0].length, greaterThanOrEqualTo(2));

      graph.close();
    });

    test('findCycles returns empty for acyclic graph', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      graph.addEdge('link', a.id, b.id);

      final cycles = graph.findCycles();
      expect(cycles, isEmpty);

      graph.close();
    });
  });

  group('Graph — Shortest path', () {
    test('shortestPath finds a path', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      final c = graph.addNode('c', {});
      graph.addEdge('link', a.id, b.id, weight: 1.0);
      graph.addEdge('link', b.id, c.id, weight: 1.0);

      final path = graph.shortestPath(a.id, c.id);
      expect(path, isNotNull);

      graph.close();
    });
  });

  group('Graph — PageRank', () {
    test('pagerank returns scores for all nodes', () {
      final graph = GraphEngine.open(bindings, tempDir.path);

      final a = graph.addNode('a', {});
      final b = graph.addNode('b', {});
      graph.addEdge('link', a.id, b.id);
      graph.addEdge('link', b.id, a.id);

      final ranks = graph.pagerank();
      expect(ranks.length, 2);
      expect(ranks.values.every((v) => v > 0), isTrue);

      graph.close();
    });
  });
}
