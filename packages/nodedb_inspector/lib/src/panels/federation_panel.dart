import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for Federation engine data.
class FederationPanel implements InspectorPanel {
  final FederationEngine _engine;

  FederationPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'federation',
        displayName: 'Federation',
        description: 'Peer and group topology',
        iconHint: 'cloud_sync',
        sortOrder: 30,
        category: 'data',
        actions: [
          PanelAction(name: 'peerList'),
          PanelAction(name: 'groupList'),
          PanelAction(name: 'peerDetail', params: [
            PanelActionParam(name: 'id', type: 'int', required: true),
          ]),
          PanelAction(name: 'topology'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'peerList':
        return peerList().map(nodePeerToJson).toList();
      case 'groupList':
        return groupList().map(nodeGroupToJson).toList();
      case 'peerDetail':
        return peerDetail(params['id'] as int);
      case 'topology':
        return topology();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown federation action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns all peers.
  List<NodePeer> peerList() => _engine.allPeers();

  /// Returns all groups.
  List<NodeGroup> groupList() => _engine.allGroups();

  /// Returns a single peer with its group memberships.
  Map<String, dynamic>? peerDetail(int id) {
    final peer = _engine.getPeer(id);
    if (peer == null) return null;
    return {
      'peer': peer,
      'groupIds': _engine.groupsForPeer(id),
    };
  }

  /// Returns the full federation topology for visualization.
  Map<String, dynamic> topology() {
    final peers = _engine.allPeers();
    final groups = _engine.allGroups();
    final memberships = <Map<String, dynamic>>[];

    for (final peer in peers) {
      final groupIds = _engine.groupsForPeer(peer.id);
      for (final gid in groupIds) {
        memberships.add({'peerId': peer.id, 'groupId': gid});
      }
    }

    return {
      'peers': peers,
      'groups': groups,
      'memberships': memberships,
    };
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    return {
      'peerCount': _engine.peerCount(),
      'groupCount': _engine.groupCount(),
    };
  }
}
