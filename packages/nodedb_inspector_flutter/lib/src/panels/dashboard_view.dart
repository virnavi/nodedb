import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Dashboard overview showing aggregate metrics from all engines.
class DashboardView extends StatelessWidget {
  final NodeDbInspector inspector;
  const DashboardView({super.key, required this.inspector});

  @override
  Widget build(BuildContext context) {
    final snap = inspector.snapshot();
    final nosql = snap['nosql'] as Map<String, dynamic>? ?? {};
    final collectionsMap = nosql['collections'];
    final collections = collectionsMap is Map ? collectionsMap.length : 0;
    final totalDocs = nosql['totalDocuments'] as int? ?? 0;
    final graph = snap['graph'] as Map<String, dynamic>?;
    final federation = snap['federation'] as Map<String, dynamic>? ?? {};
    final triggers = snap['triggers'] as Map<String, dynamic>? ?? {};
    final singletons = snap['singletons'] as Map<String, dynamic>? ?? {};
    final history = snap['accessHistory'] as Map<String, dynamic>? ?? {};

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const SectionHeader(title: 'Overview'),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Collections',
              value: '$collections',
              icon: Icons.storage,
            ),
            MetricCard(
              label: 'Documents',
              value: '$totalDocs',
              icon: Icons.description,
            ),
            if (graph != null)
              MetricCard(
                label: 'Graph Nodes',
                value: '${graph['nodeCount'] ?? 0}',
                icon: Icons.hub,
                valueColor: InspectorColors.green,
              ),
            MetricCard(
              label: 'Peers',
              value: '${federation['peerCount'] ?? 0}',
              icon: Icons.cloud_sync,
              valueColor: InspectorColors.cyan,
            ),
            MetricCard(
              label: 'Triggers',
              value: '${triggers['totalTriggers'] ?? 0}',
              icon: Icons.flash_on,
              valueColor: InspectorColors.yellow,
            ),
            MetricCard(
              label: 'Singletons',
              value: '${singletons['count'] ?? 0}',
              icon: Icons.tune,
              valueColor: InspectorColors.magenta,
            ),
            MetricCard(
              label: 'Access Events',
              value: '${history['totalEntries'] ?? 0}',
              icon: Icons.history,
              valueColor: InspectorColors.textDim,
            ),
          ],
        ),
        const SizedBox(height: 24),
        const SectionHeader(title: 'Enabled Panels'),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final name in inspector.enabledPanels())
              Chip(
                label: Text(name, style: const TextStyle(fontSize: 11)),
                backgroundColor: InspectorColors.surface,
                side: const BorderSide(color: InspectorColors.border),
              ),
          ],
        ),
        if (snap.containsKey('provenance')) ...[
          const SizedBox(height: 24),
          const SectionHeader(title: 'Provenance'),
          _provenanceRow(snap['provenance'] as Map<String, dynamic>),
        ],
        if (snap.containsKey('ai')) ...[
          const SizedBox(height: 24),
          const SectionHeader(title: 'AI'),
          _aiRow(snap['ai'] as Map<String, dynamic>),
        ],
      ],
    );
  }

  Widget _provenanceRow(Map<String, dynamic> prov) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        MetricCard(
          label: 'Envelopes',
          value: '${prov['envelopeCount'] ?? 0}',
          icon: Icons.verified,
          valueColor: InspectorColors.green,
        ),
      ],
    );
  }

  Widget _aiRow(Map<String, dynamic> ai) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: [
        MetricCard(
          label: 'AI Augmented',
          value: '${ai['aiAugmented'] ?? 0}',
          icon: Icons.psychology,
          valueColor: InspectorColors.magenta,
        ),
        MetricCard(
          label: 'AI Originated',
          value: '${ai['aiOriginated'] ?? 0}',
          icon: Icons.auto_awesome,
          valueColor: InspectorColors.cyan,
        ),
        MetricCard(
          label: 'Anomalies',
          value: '${ai['anomalyFlagged'] ?? 0}',
          icon: Icons.warning,
          valueColor: InspectorColors.red,
        ),
      ],
    );
  }
}
