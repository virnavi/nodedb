import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Provenance panel: envelope browser, confidence histogram, stats.
class ProvenanceView extends StatelessWidget {
  final ProvenancePanel panel;
  const ProvenanceView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final stats = panel.stats();
    final recent = panel.recentEnvelopes(limit: 30);
    final sourceBreakdown =
        stats['sourceTypeBreakdown'] as Map<String, dynamic>? ?? {};
    final verifyBreakdown =
        stats['verificationBreakdown'] as Map<String, dynamic>? ?? {};

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Total Envelopes',
              value: '${stats['totalCount'] ?? 0}',
              icon: Icons.verified,
            ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Source Types'),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            for (final entry in sourceBreakdown.entries)
              MetricCard(
                label: entry.key,
                value: '${entry.value}',
                valueColor: InspectorColors.cyan,
              ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Verification Status'),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            for (final entry in verifyBreakdown.entries)
              MetricCard(
                label: entry.key,
                value: '${entry.value}',
                valueColor: _verifyColor(entry.key),
              ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Recent Envelopes'),
        if (recent.isEmpty)
          const EmptyState(message: 'No provenance envelopes')
        else
          for (final env in recent) _envelopeTile(env),
      ],
    );
  }

  Color _verifyColor(String status) {
    switch (status) {
      case 'verified':
        return InspectorColors.green;
      case 'failed':
        return InspectorColors.red;
      case 'key_requested':
        return InspectorColors.yellow;
      case 'trust_all':
        return InspectorColors.magenta;
      default:
        return InspectorColors.textDim;
    }
  }

  Widget _envelopeTile(ProvenanceEnvelope env) {
    final json = provenanceEnvelopeToJson(env);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Text(
                  '${json['collection']}:${json['recordId']}',
                  style: const TextStyle(
                    color: InspectorColors.accent,
                    fontSize: 12,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const Spacer(),
                _confidenceBadge(json['confidenceFactor'] as double? ?? 0),
              ],
            ),
            KeyValueRow(label: 'Source', value: '${json['sourceId']}'),
            KeyValueRow(label: 'Type', value: '${json['sourceType']}'),
            KeyValueRow(
                label: 'Status', value: '${json['verificationStatus']}'),
            if (json['pkiId'] != null)
              KeyValueRow(label: 'PKI ID', value: '${json['pkiId']}'),
          ],
        ),
      ),
    );
  }

  Widget _confidenceBadge(double confidence) {
    final color = confidence >= 0.8
        ? InspectorColors.green
        : confidence >= 0.5
            ? InspectorColors.yellow
            : InspectorColors.red;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: color.withAlpha(30),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color, width: 1),
      ),
      child: Text(
        confidence.toStringAsFixed(2),
        style: TextStyle(color: color, fontSize: 11, fontWeight: FontWeight.bold),
      ),
    );
  }
}
