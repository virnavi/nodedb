import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/graph_node.dart';
import '../model/graph_edge.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB Graph engine.
class GraphEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  GraphEngine._(this._handle, this._bindings);

  /// Attach to an existing graph engine handle (for multi-isolate use).
  factory GraphEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return GraphEngine._(handle, bindings);
  }

  static GraphEngine open(NodeDbBindings bindings, String path) {
    final config = buildConfig(path);
    final handle = openRaw(bindings, bindings.graphOpen, config);
    return GraphEngine._(handle, bindings);
  }

  int get handle => _handle;

  // ── Node operations ─────────────────────────────────────────────

  GraphNode addNode(String label, Map<String, dynamic> data) {
    final resp = _execute({'action': 'add_node', 'label': label, 'data': data});
    return GraphNode.fromMsgpack(resp);
  }

  /// Get a node by ID. Returns null if the node does not exist.
  GraphNode? getNode(int id) {
    try {
      final resp = _execute({'action': 'get_node', 'id': id});
      if (resp == null) return null;
      return GraphNode.fromMsgpack(resp);
    } on NodeDbException {
      return null;
    }
  }

  GraphNode updateNode(int id, Map<String, dynamic> data) {
    final resp = _execute({'action': 'update_node', 'id': id, 'data': data});
    return GraphNode.fromMsgpack(resp);
  }

  void deleteNode(int id, {String behaviour = 'detach'}) {
    _execute({'action': 'delete_node', 'id': id, 'behaviour': behaviour});
  }

  List<GraphNode> allNodes() {
    final resp = _execute({'action': 'all_nodes'});
    if (resp is! List) return [];
    return resp.map((n) => GraphNode.fromMsgpack(n)).toList();
  }

  int nodeCount() {
    final resp = _execute({'action': 'node_count'});
    return (resp is int) ? resp : 0;
  }

  // ── Edge operations ─────────────────────────────────────────────

  GraphEdge addEdge(
    String label,
    int source,
    int target, {
    double weight = 1.0,
    Map<String, dynamic>? data,
  }) {
    final resp = _execute({
      'action': 'add_edge',
      'label': label,
      'source': source,
      'target': target,
      'weight': weight,
      if (data != null) 'data': data,
    });
    return GraphEdge.fromMsgpack(resp);
  }

  GraphEdge? getEdge(int id) {
    final resp = _execute({'action': 'get_edge', 'id': id});
    if (resp == null) return null;
    return GraphEdge.fromMsgpack(resp);
  }

  GraphEdge updateEdge(int id, Map<String, dynamic> data) {
    final resp = _execute({
      'action': 'update_edge',
      'id': id,
      'data': data,
    });
    return GraphEdge.fromMsgpack(resp);
  }

  void deleteEdge(int id) {
    _execute({'action': 'delete_edge', 'id': id});
  }

  List<GraphEdge> edgesFrom(int nodeId) {
    final resp = _execute({'action': 'edges_from', 'id': nodeId});
    if (resp is! List) return [];
    return resp.map((e) => GraphEdge.fromMsgpack(e)).toList();
  }

  List<GraphEdge> edgesTo(int nodeId) {
    final resp = _execute({'action': 'edges_to', 'id': nodeId});
    if (resp is! List) return [];
    return resp.map((e) => GraphEdge.fromMsgpack(e)).toList();
  }

  // ── Algorithms ──────────────────────────────────────────────────

  /// BFS traversal from [startId]. Returns a map with "nodes" (List<int>)
  /// and "edges" (List<int>) keys containing IDs visited.
  Map<String, List<int>> bfs(int startId, {int maxDepth = 10}) {
    final resp = _execute({
      'action': 'bfs',
      'id': startId,
      'max_depth': maxDepth,
    });
    return _parseTraversalResult(resp);
  }

  /// DFS traversal from [startId]. Returns a map with "nodes" (List<int>)
  /// and "edges" (List<int>) keys containing IDs visited.
  Map<String, List<int>> dfs(int startId, {int maxDepth = 10}) {
    final resp = _execute({
      'action': 'dfs',
      'id': startId,
      'max_depth': maxDepth,
    });
    return _parseTraversalResult(resp);
  }

  Map<String, List<int>> _parseTraversalResult(dynamic resp) {
    if (resp is Map) {
      final nodes = (resp['nodes'] as List?)
              ?.map((e) => (e as num).toInt())
              .toList() ??
          [];
      final edges = (resp['edges'] as List?)
              ?.map((e) => (e as num).toInt())
              .toList() ??
          [];
      return {'nodes': nodes, 'edges': edges};
    }
    return {'nodes': [], 'edges': []};
  }

  dynamic shortestPath(int from, int to) {
    return _execute({'action': 'shortest_path', 'from': from, 'to': to});
  }

  Map<int, double> pagerank({double damping = 0.85, int iterations = 20}) {
    final resp = _execute({
      'action': 'pagerank',
      'damping': damping,
      'iterations': iterations,
    });
    if (resp is Map) {
      return resp.map((k, v) => MapEntry(
        (k is num) ? k.toInt() : int.parse(k.toString()),
        (v is num) ? v.toDouble() : 0.0,
      ));
    }
    return {};
  }

  /// Get neighbor nodes for a given node.
  List<GraphNode> neighbors(int nodeId) {
    final resp = _execute({'action': 'neighbors', 'id': nodeId});
    if (resp is! List) return [];
    return resp.map((n) => GraphNode.fromMsgpack(n)).toList();
  }

  /// Find connected components. Returns list of node ID lists.
  List<List<int>> connectedComponents() {
    final resp = _execute({'action': 'connected_components'});
    if (resp is! List) return [];
    return resp
        .map((c) => (c as List).map((id) => (id as num).toInt()).toList())
        .toList();
  }

  /// Check if the graph contains any cycle.
  bool hasCycle() {
    final resp = _execute({'action': 'has_cycle'});
    if (resp is Map) return resp['has_cycle'] == true;
    return resp == true;
  }

  /// Find all cycles in the graph. Returns list of node ID lists.
  List<List<int>> findCycles() {
    final resp = _execute({'action': 'find_cycles'});
    if (resp is! List) return [];
    return resp
        .map((c) => (c as List).map((id) => (id as num).toInt()).toList())
        .toList();
  }

  void close() {
    _bindings.graphClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.graphExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
