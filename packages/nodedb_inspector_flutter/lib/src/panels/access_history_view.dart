import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Access history panel: heatmap and event log.
class AccessHistoryView extends StatelessWidget {
  final AccessHistoryPanel panel;
  const AccessHistoryView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final summary = panel.summary();
    final totalEntries = summary['totalEntries'] as int? ?? 0;
    final heatmap = summary['heatmap'] as Map<String, dynamic>? ?? {};
    final recent = panel.query();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Total Events',
              value: '$totalEntries',
              icon: Icons.history,
            ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Collection Heatmap'),
        if (heatmap.isEmpty)
          const Padding(
            padding: EdgeInsets.all(16),
            child: Text('No access history', style: TextStyle(
              color: InspectorColors.textDim, fontSize: 12)),
          )
        else
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              for (final entry in heatmap.entries)
                _heatmapCard(entry.key, entry.value as int),
            ],
          ),
        const SizedBox(height: 16),
        SectionHeader(title: 'Recent Events (${recent.length})'),
        if (recent.isEmpty)
          const EmptyState(message: 'No events recorded')
        else
          for (final event in recent.take(50)) _eventTile(event),
      ],
    );
  }

  Widget _heatmapCard(String collection, int count) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(collection, style: const TextStyle(
              color: InspectorColors.text, fontSize: 11)),
            const SizedBox(height: 4),
            Text('$count', style: const TextStyle(
              color: InspectorColors.cyan,
              fontSize: 20,
              fontWeight: FontWeight.bold,
            )),
          ],
        ),
      ),
    );
  }

  Widget _eventTile(Map<String, dynamic> event) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          SizedBox(
            width: 80,
            child: Text(
              '${event['eventType'] ?? '?'}',
              style: TextStyle(
                color: _eventColor('${event['eventType']}'),
                fontSize: 11,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          Expanded(
            child: Text(
              '${event['collection'] ?? '?'}${event['recordId'] != null ? ':${event['recordId']}' : ''}',
              style: const TextStyle(
                color: InspectorColors.text, fontSize: 11),
            ),
          ),
          Text(
            '${event['timestamp'] ?? ''}',
            style: const TextStyle(
              color: InspectorColors.textDim, fontSize: 10),
          ),
        ],
      ),
    );
  }

  Color _eventColor(String type) {
    switch (type) {
      case 'read':
        return InspectorColors.green;
      case 'write':
        return InspectorColors.accent;
      case 'watch':
        return InspectorColors.cyan;
      case 'federatedRead':
        return InspectorColors.magenta;
      case 'aiWrite':
        return InspectorColors.yellow;
      default:
        return InspectorColors.textDim;
    }
  }
}
