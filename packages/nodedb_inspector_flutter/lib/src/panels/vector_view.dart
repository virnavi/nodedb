import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import 'panel_helpers.dart';

/// Vector panel: stats and search interface.
class VectorView extends StatelessWidget {
  final VectorPanel panel;
  const VectorView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final stats = panel.stats();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const SectionHeader(title: 'Vector Engine'),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(label: 'Records', value: '${stats['count'] ?? 0}'),
            MetricCard(label: 'Dimension', value: '${stats['dimension'] ?? 0}'),
            MetricCard(label: 'Metric', value: '${stats['metric'] ?? 'unknown'}'),
            MetricCard(
                label: 'Max Elements',
                value: '${stats['maxElements'] ?? 0}'),
          ],
        ),
      ],
    );
  }
}
