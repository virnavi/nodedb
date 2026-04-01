import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import '../inspector_theme.dart';
import 'panel_helpers.dart';

/// Federation panel: peers, groups, topology.
class FederationView extends StatelessWidget {
  final FederationPanel panel;
  const FederationView({super.key, required this.panel});

  @override
  Widget build(BuildContext context) {
    final summary = panel.summary();
    final peers = panel.peerList();
    final groups = panel.groupList();

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            MetricCard(
              label: 'Peers',
              value: '${summary['peerCount'] ?? 0}',
              icon: Icons.people,
              valueColor: InspectorColors.cyan,
            ),
            MetricCard(
              label: 'Groups',
              value: '${summary['groupCount'] ?? 0}',
              icon: Icons.group_work,
              valueColor: InspectorColors.magenta,
            ),
          ],
        ),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Peers'),
        if (peers.isEmpty)
          const Padding(
            padding: EdgeInsets.all(16),
            child: Text('No peers', style: TextStyle(
              color: InspectorColors.textDim, fontSize: 12)),
          )
        else
          for (final peer in peers)
            _peerTile(peer),
        const SizedBox(height: 16),
        const SectionHeader(title: 'Groups'),
        if (groups.isEmpty)
          const Padding(
            padding: EdgeInsets.all(16),
            child: Text('No groups', style: TextStyle(
              color: InspectorColors.textDim, fontSize: 12)),
          )
        else
          for (final group in groups)
            _groupTile(group),
      ],
    );
  }

  Widget _peerTile(NodePeer peer) {
    final json = nodePeerToJson(peer);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${json['name']}', style: const TextStyle(
              color: InspectorColors.accent, fontSize: 12,
              fontWeight: FontWeight.bold)),
            if (json['endpoint'] != null)
              KeyValueRow(label: 'Endpoint', value: '${json['endpoint']}'),
            if (json['status'] != null)
              KeyValueRow(label: 'Status', value: '${json['status']}'),
          ],
        ),
      ),
    );
  }

  Widget _groupTile(NodeGroup group) {
    final json = nodeGroupToJson(group);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('${json['name']}', style: const TextStyle(
              color: InspectorColors.magenta, fontSize: 12,
              fontWeight: FontWeight.bold)),
            KeyValueRow(
              label: 'Members',
              value: '${(json['members'] as List?)?.length ?? 0}',
            ),
          ],
        ),
      ),
    );
  }
}
