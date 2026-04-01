import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Trigger panel: registered triggers, grouped by collection.
class TriggerView extends StatelessWidget {
  final TriggerPanel panel;
  const TriggerView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final summary = panel.summary();
    final total = summary['totalTriggers'] as int? ?? 0;
    final enabled = summary['enabled'] as int? ?? 0;
    final disabled = summary['disabled'] as int? ?? 0;
    final byCollection = panel.triggersByCollection();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(label: 'Total', value: '$total', icon: Icons.flash_on),
            MetricCard(
              label: 'Enabled',
              value: '$enabled',
              valueColor: InspectorColors.green,
            ),
            MetricCard(
              label: 'Disabled',
              value: '$disabled',
              valueColor: InspectorColors.red,
            ),
          ],
        ),
        const SizedBox(height: 16),
        for (final entry in byCollection.entries) ...[
          SectionHeader(title: entry.key),
          for (final trigger in entry.value) _triggerTile(trigger),
        ],
        if (byCollection.isEmpty)
          const EmptyState(message: 'No triggers registered'),
      ],
    );
  }

  Widget _triggerTile(Map<String, dynamic> trigger) {
    final isEnabled = trigger['enabled'] as bool? ?? true;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: [
            Icon(
              isEnabled ? Icons.check_circle : Icons.cancel,
              size: 16,
              color: isEnabled ? InspectorColors.green : InspectorColors.red,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    trigger['name'] as String? ?? 'unnamed',
                    style: const TextStyle(
                      color: InspectorColors.text,
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  Text(
                    '${trigger['event']} / ${trigger['timing']}',
                    style: const TextStyle(
                      color: InspectorColors.textDim, fontSize: 11),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
