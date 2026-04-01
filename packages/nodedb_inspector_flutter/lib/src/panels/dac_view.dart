import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// DAC panel: access rules browser.
class DacView extends StatelessWidget {
  final DacPanel panel;
  const DacView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final summary = panel.summary();
    final rules = panel.ruleList();
    final stats = panel.ruleStats();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Total Rules',
              value: '${summary['ruleCount'] ?? 0}',
              icon: Icons.security,
            ),
            for (final entry in stats.entries)
              MetricCard(
                label: entry.key,
                value: '${entry.value}',
                valueColor: _permColor(entry.key),
              ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Access Rules'),
        if (rules.isEmpty)
          const EmptyState(message: 'No access rules defined')
        else
          for (final rule in rules) _ruleTile(rule),
      ],
    );
  }

  Color _permColor(String perm) {
    switch (perm.toLowerCase()) {
      case 'allow':
        return InspectorColors.green;
      case 'deny':
        return InspectorColors.red;
      case 'redact':
        return InspectorColors.yellow;
      default:
        return InspectorColors.textDim;
    }
  }

  Widget _ruleTile(AccessRule rule) {
    final json = accessRuleToJson(rule);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text(
                  '${json['permission']}'.toUpperCase(),
                  style: TextStyle(
                    color: _permColor('${json['permission']}'),
                    fontSize: 11,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${json['collection']}${json['field'] != null ? '.${json['field']}' : ''}',
                    style: const TextStyle(
                      color: InspectorColors.text, fontSize: 12),
                  ),
                ),
              ],
            ),
            KeyValueRow(
              label: 'Subject',
              value: '${json['subjectType']}:${json['subjectId']}',
            ),
          ],
        ),
      ),
    );
  }
}
