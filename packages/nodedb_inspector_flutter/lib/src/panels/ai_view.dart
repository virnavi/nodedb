import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// AI panel: AI provenance stats, anomalies, config.
class AiView extends StatelessWidget {
  final AiPanel panel;
  const AiView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final stats = panel.stats();
    final provConfig = panel.aiProvenanceConfig();
    final queryConfig = panel.aiQueryConfig();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const SectionHeader(title: 'AI Statistics'),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Total Envelopes',
              value: '${stats['totalEnvelopes'] ?? 0}',
              icon: Icons.description,
            ),
            MetricCard(
              label: 'AI Augmented',
              value: '${stats['aiAugmented'] ?? 0}',
              icon: Icons.psychology,
              valueColor: InspectorColors.magenta,
            ),
            MetricCard(
              label: 'AI Originated',
              value: '${stats['aiOriginated'] ?? 0}',
              icon: Icons.auto_awesome,
              valueColor: InspectorColors.cyan,
            ),
            MetricCard(
              label: 'Anomaly Flagged',
              value: '${stats['anomalyFlagged'] ?? 0}',
              icon: Icons.warning,
              valueColor: InspectorColors.red,
            ),
          ],
        ),
        if (stats['anomalySeverity'] is Map) ...[
          const SizedBox(height: 16),
          const SectionHeader(title: 'Anomaly Severity'),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              for (final entry
                  in (stats['anomalySeverity'] as Map).entries)
                MetricCard(
                  label: '${entry.key}',
                  value: '${entry.value}',
                  valueColor: InspectorColors.yellow,
                ),
            ],
          ),
        ],
        const SizedBox(height: 16),
        const SectionHeader(title: 'AI Provenance Config'),
        _configCard(provConfig),
        const SizedBox(height: 16),
        const SectionHeader(title: 'AI Query Config'),
        _configCard(queryConfig),
      ],
    );
  }

  Widget _configCard(Map<String, dynamic> config) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final entry in config.entries)
              KeyValueRow(label: entry.key, value: '${entry.value}'),
            if (config.isEmpty)
              const Text('Not configured', style: TextStyle(
                color: InspectorColors.textDim, fontSize: 12)),
          ],
        ),
      ),
    );
  }
}
