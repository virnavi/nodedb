import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for Graph engine data.
class GraphPanel implements InspectorPanel {
  final GraphEngine? _engine;

  GraphPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'graph',
        displayName: 'Graph',
        description: 'Graph nodes, edges, and traversals',
        iconHint: 'hub',
        sortOrder: 20,
        category: 'data',
        actions: [
          PanelAction(name: 'stats'),
          PanelAction(name: 'nodePreview', params: [
            PanelActionParam(name: 'limit', type: 'int'),
          ]),
          PanelAction(name: 'nodeDetail', params: [
            PanelActionParam(name: 'id', type: 'int', required: true),
          ]),
          PanelAction(name: 'traversal', params: [
            PanelActionParam(name: 'startId', type: 'int', required: true),
            PanelActionParam(name: 'algorithm', type: 'string', required: true),
            PanelActionParam(name: 'maxDepth', type: 'int'),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _engine != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'stats':
        return stats();
      case 'nodePreview':
        return nodePreview(limit: params['limit'] as int? ?? 50)
            .map(graphNodeToJson)
            .toList();
      case 'nodeDetail':
        return nodeDetail(params['id'] as int);
      case 'traversal':
        return traversal(
          params['startId'] as int,
          params['algorithm'] as String,
          maxDepth: params['maxDepth'] as int? ?? 10,
        );
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown graph action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns node count.
  Map<String, int> stats() {
    return {
      'nodeCount': _engine!.nodeCount(),
    };
  }

  /// Returns a preview of graph nodes.
  List<GraphNode> nodePreview({int limit = 50}) {
    final all = _engine!.allNodes();
    if (all.length <= limit) return all;
    return all.sublist(0, limit);
  }

  /// Returns a single node with its edges.
  Map<String, dynamic>? nodeDetail(int id) {
    final node = _engine!.getNode(id);
    if (node == null) return null;
    return {
      'node': node,
      'edgesFrom': _engine!.edgesFrom(id),
      'edgesTo': _engine!.edgesTo(id),
    };
  }

  /// Runs a graph traversal algorithm.
  Map<String, List<int>> traversal(
    int startId,
    String algorithm, {
    int maxDepth = 10,
  }) {
    switch (algorithm) {
      case 'bfs':
        return _engine!.bfs(startId, maxDepth: maxDepth);
      case 'dfs':
        return _engine!.dfs(startId, maxDepth: maxDepth);
      default:
        return {'nodes': [], 'edges': []};
    }
  }

  @override
  Map<String, dynamic> summary() => stats().cast<String, dynamic>();
}
