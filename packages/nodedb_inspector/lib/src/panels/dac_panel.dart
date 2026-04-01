import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for DAC (Data Access Control) engine data.
class DacPanel implements InspectorPanel {
  final DacEngine? _engine;

  DacPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'dac',
        displayName: 'DAC',
        description: 'Data access control rules',
        iconHint: 'security',
        sortOrder: 40,
        category: 'security',
        actions: [
          PanelAction(name: 'ruleList', params: [
            PanelActionParam(name: 'collection', type: 'string'),
          ]),
          PanelAction(name: 'ruleStats'),
          PanelAction(name: 'testAccess', params: [
            PanelActionParam(name: 'collection', type: 'string', required: true),
            PanelActionParam(name: 'peerId', type: 'string', required: true),
            PanelActionParam(name: 'document', type: 'map', required: true),
            PanelActionParam(name: 'groupIds', type: 'list'),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _engine != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'ruleList':
        return ruleList(collection: params['collection'] as String?)
            .map(accessRuleToJson)
            .toList();
      case 'ruleStats':
        return ruleStats();
      case 'testAccess':
        return testAccess(
          collection: params['collection'] as String,
          peerId: params['peerId'] as String,
          document: Map<String, dynamic>.from(params['document'] as Map),
          groupIds: (params['groupIds'] as List?)?.cast<String>(),
        );
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown dac action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns all rules, optionally filtered by collection.
  List<AccessRule> ruleList({String? collection}) {
    if (collection != null) return _engine!.rulesForCollection(collection);
    return _engine!.allRules();
  }

  /// Returns rule count broken down by permission type.
  Map<String, int> ruleStats() {
    final rules = _engine!.allRules();
    final stats = <String, int>{};
    for (final rule in rules) {
      stats[rule.permission] = (stats[rule.permission] ?? 0) + 1;
    }
    return stats;
  }

  /// Tests document access for a given peer.
  Map<String, dynamic>? testAccess({
    required String collection,
    required String peerId,
    required Map<String, dynamic> document,
    List<String>? groupIds,
  }) {
    return _engine!.filterDocument(
      collection: collection,
      document: document,
      peerId: peerId,
      groupIds: groupIds,
    );
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    return {
      'ruleCount': _engine!.ruleCount(),
    };
  }
}
