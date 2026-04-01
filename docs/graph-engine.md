# Graph Engine

[← Back to Index](README.md)

The graph engine provides a labeled property graph with nodes, weighted edges, and built-in graph algorithms.

## Core Concepts

### Nodes

```dart
class GraphNode {
  final int id;
  final String label;              // e.g., 'Person', 'City'
  final Map<String, dynamic> data; // Arbitrary properties
}
```

### Edges

```dart
class GraphEdge {
  final int id;
  final String label;              // e.g., 'KNOWS', 'LIVES_IN'
  final int source;                // Source node ID
  final int target;                // Target node ID
  final double weight;             // Edge weight (immutable after creation)
  final Map<String, dynamic> data; // Arbitrary properties
}
```

## CRUD Operations

```dart
final graph = db.graph!;

// Add nodes
final alice = graph.addNode('Person', data: {'name': 'Alice', 'age': 30});
final bob = graph.addNode('Person', data: {'name': 'Bob', 'age': 25});
final city = graph.addNode('City', data: {'name': 'Berlin'});

// Add edges
graph.addEdge('KNOWS', alice.id, bob.id, weight: 1.0, data: {'since': 2020});
graph.addEdge('LIVES_IN', alice.id, city.id, weight: 0.0);

// Read
final node = graph.getNode(alice.id);
final edge = graph.getEdge(edgeId);

// Update (data only — weight is immutable)
graph.updateNode(alice.id, data: {'name': 'Alice', 'age': 31});
graph.updateEdge(edgeId, data: {'since': 2019});

// Delete
graph.deleteNode(bob.id, deleteBehaviour: 'detach'); // Also removes edges
graph.deleteEdge(edgeId);
```

### Delete Behaviours

| Behaviour | Effect |
|-----------|--------|
| `detach` | Remove all connected edges, then delete the node |
| `restrict` | Fail if the node has any edges |
| `cascade` | Delete connected nodes recursively |
| `nullify` | Set edge source/target to null (not commonly used) |

## Graph Algorithms

### Traversal

```dart
// Breadth-first search from a starting node
final bfsResult = graph.bfs(startNodeId, maxDepth: 3);
// Returns: {nodes: [int], edges: [int]}

// Depth-first search
final dfsResult = graph.dfs(startNodeId, maxDepth: 5);

// Shortest path (weight-aware)
final path = graph.shortestPath(fromNodeId, toNodeId);

// Multi-hop traversal
final multiHop = graph.multiHop(startNodeId, hops: 3);

// Neighbors of a node
final neighbors = graph.neighbors(nodeId);
// Returns: List<GraphNode>
```

### Analysis

```dart
// PageRank — returns {nodeId: score} map
final ranks = graph.pagerank(
  dampingFactor: 0.85,
  iterations: 100,
);
// Map<int, double>, e.g., {1: 0.35, 2: 0.28, ...}

// Connected components — groups of reachable nodes
final components = graph.connectedComponents();
// List<List<int>>, e.g., [[1, 2, 3], [4, 5]]

// Cycle detection
final hasCycle = graph.hasCycle(); // bool

// Find all cycles
final cycles = graph.findCycles();
// List<List<int>>, e.g., [[1, 2, 3], [4, 5, 6]]
```

## Storage Model

The graph uses three sled trees:

- **Nodes tree** — `nodeId → GraphNode` (msgpack)
- **Edges tree** — `edgeId → GraphEdge` (msgpack)
- **Adjacency out** — `sourceId → [edgeId, ...]` (outgoing edges)
- **Adjacency in** — `targetId → [edgeId, ...]` (incoming edges)

Separate ID generators for nodes and edges.

## Code Generation

Use `@node` and `@Edge` annotations:

```dart
@node
class Person {
  String name;
  int age;
  Person({required this.name, this.age = 0});
}

@Edge(from: Person, to: Person)
class Knows {
  int since;
  Knows({required this.since});
}
```

This generates typed DAOs and serialization for graph entities.

## Rust Implementation

**Crate**: `nodedb-graph` → depends on `nodedb-storage`

Key types:
- `GraphEngine` — main CRUD + algorithms interface
- `GraphNode`, `GraphEdge` — data models
- `DeleteBehaviour` — node deletion strategy
- `TraversalResult` — BFS/DFS/shortest path result

## Related Pages

- [NoSQL Engine](nosql-engine.md) — document storage (complementary)
- [Code Generation](code-generation.md) — `@node` and `@Edge` annotations
- [Getting Started](getting-started.md) — enabling the graph engine
