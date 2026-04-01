import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Graph panel: node list, edge details, BFS/DFS traversal.
class GraphView extends StatefulWidget {
  final GraphPanel panel;
  const GraphView({super.key, required this.panel});

  @override
  State<GraphView> createState() => _GraphViewState();
}

class _GraphViewState extends State<GraphView> {
  int? _selectedNodeId;
  Map<String, List<int>>? _traversalResult;
  String _traversalLabel = '';

  @override
  Widget build(BuildContext context) {
    final stats = widget.panel.stats();
    final nodes = widget.panel.nodePreview();

    return Row(
      children: [
        // Node list
        SizedBox(
          width: 260,
          child: Column(
            children: [
              SectionHeader(title: 'Nodes (${stats['nodeCount'] ?? 0})'),
              Expanded(
                child: nodes.isEmpty
                    ? const EmptyState(message: 'No graph nodes')
                    : ListView.builder(
                        itemCount: nodes.length,
                        itemBuilder: (ctx, i) {
                          final node = nodes[i];
                          final selected = node.id == _selectedNodeId;
                          return ListTile(
                            dense: true,
                            selected: selected,
                            selectedTileColor: InspectorColors.surface,
                            leading: CircleAvatar(
                              radius: 14,
                              backgroundColor: InspectorColors.accent,
                              foregroundColor: InspectorColors.bg,
                              child: Text('${node.id}',
                                  style: const TextStyle(fontSize: 10)),
                            ),
                            title: Text(node.label,
                                style: const TextStyle(fontSize: 12)),
                            trailing: PopupMenuButton<String>(
                              iconSize: 18,
                              onSelected: (action) =>
                                  _onAction(action, node.id),
                              itemBuilder: (_) => const [
                                PopupMenuItem(
                                    value: 'detail', child: Text('Detail')),
                                PopupMenuItem(
                                    value: 'bfs', child: Text('BFS')),
                                PopupMenuItem(
                                    value: 'dfs', child: Text('DFS')),
                              ],
                            ),
                            onTap: () => _onAction('detail', node.id),
                          );
                        },
                      ),
              ),
            ],
          ),
        ),
        const VerticalDivider(width: 1),
        // Detail / traversal
        Expanded(
          child: _selectedNodeId == null
              ? const EmptyState(
                  message: 'Select a node', icon: Icons.touch_app)
              : _buildDetail(),
        ),
      ],
    );
  }

  void _onAction(String action, int nodeId) {
    setState(() {
      _selectedNodeId = nodeId;
      if (action == 'bfs' || action == 'dfs') {
        _traversalResult = widget.panel.traversal(nodeId, action);
        _traversalLabel = '${action.toUpperCase()} from $nodeId';
      } else {
        _traversalResult = null;
        _traversalLabel = '';
      }
    });
  }

  Widget _buildDetail() {
    final detail = widget.panel.nodeDetail(_selectedNodeId!);
    if (detail == null) {
      return const EmptyState(message: 'Node not found');
    }

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        SectionHeader(title: 'Node ${detail['id']}'),
        KeyValueRow(label: 'Label', value: '${detail['label']}'),
        if (detail['data'] is Map)
          for (final e in (detail['data'] as Map).entries)
            KeyValueRow(label: '${e.key}', value: '${e.value}'),
        const SizedBox(height: 16),
        SectionHeader(title: 'Edges Out (${(detail['edgesFrom'] as List?)?.length ?? 0})'),
        if (detail['edgesFrom'] is List)
          for (final edge in detail['edgesFrom'] as List)
            _edgeRow(edge as Map<String, dynamic>),
        const SizedBox(height: 8),
        SectionHeader(title: 'Edges In (${(detail['edgesTo'] as List?)?.length ?? 0})'),
        if (detail['edgesTo'] is List)
          for (final edge in detail['edgesTo'] as List)
            _edgeRow(edge as Map<String, dynamic>),
        if (_traversalResult != null) ...[
          const SizedBox(height: 16),
          SectionHeader(title: _traversalLabel),
          KeyValueRow(
            label: 'Visited',
            value: (_traversalResult!['nodes'] ?? []).join(' -> '),
          ),
          KeyValueRow(
            label: 'Edges',
            value: (_traversalResult!['edges'] ?? []).join(', '),
          ),
        ],
      ],
    );
  }

  Widget _edgeRow(Map<String, dynamic> edge) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 2),
      child: Text(
        '${edge['source']} -> ${edge['target']} (w: ${edge['weight']})',
        style: const TextStyle(
          color: InspectorColors.text, fontSize: 12),
      ),
    );
  }
}
