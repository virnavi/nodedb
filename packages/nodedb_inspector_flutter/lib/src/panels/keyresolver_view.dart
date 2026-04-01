import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Key resolver panel: cached public keys, trust levels.
class KeyResolverView extends StatelessWidget {
  final KeyResolverPanel panel;
  const KeyResolverView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final stats = panel.keyStats();
    final keys = panel.keyList();
    final breakdown =
        stats['trustLevelBreakdown'] as Map<String, dynamic>? ?? {};
    final trustAll = stats['trustAllActive'] as bool? ?? false;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Cached Keys',
              value: '${stats['totalKeys'] ?? 0}',
              icon: Icons.vpn_key,
            ),
            MetricCard(
              label: 'Trust-All',
              value: trustAll ? 'ON' : 'OFF',
              icon: Icons.shield,
              valueColor:
                  trustAll ? InspectorColors.yellow : InspectorColors.green,
            ),
            for (final entry in breakdown.entries)
              MetricCard(
                label: entry.key,
                value: '${entry.value}',
                valueColor: _trustColor(entry.key),
              ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Key Cache'),
        if (keys.isEmpty)
          const EmptyState(message: 'No cached keys')
        else
          for (final key in keys) _keyTile(key),
      ],
    );
  }

  Color _trustColor(String level) {
    switch (level) {
      case 'explicit':
        return InspectorColors.green;
      case 'trust_all':
        return InspectorColors.yellow;
      case 'revoked':
        return InspectorColors.red;
      default:
        return InspectorColors.textDim;
    }
  }

  Widget _keyTile(KeyEntry key) {
    final json = keyEntryToJson(key);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '${json['pkiId']}',
              style: const TextStyle(
                color: InspectorColors.accent,
                fontSize: 12,
                fontWeight: FontWeight.bold,
              ),
            ),
            KeyValueRow(label: 'User', value: '${json['userId']}'),
            KeyValueRow(label: 'Trust', value: '${json['trustLevel']}'),
            KeyValueRow(
              label: 'Public Key',
              value: _truncateHex('${json['publicKeyHex']}'),
            ),
          ],
        ),
      ),
    );
  }

  String _truncateHex(String hex) {
    if (hex.length <= 16) return hex;
    return '${hex.substring(0, 8)}...${hex.substring(hex.length - 8)}';
  }
}
