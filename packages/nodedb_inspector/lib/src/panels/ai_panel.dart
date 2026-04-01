import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for AI provenance augmentation and AI query data.
class AiPanel implements InspectorPanel {
  final ProvenanceEngine? _provenance;
  final AiProvenanceEngine? _aiProvenance;
  final AiQueryEngine? _aiQuery;

  AiPanel(this._provenance, this._aiProvenance, this._aiQuery);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'ai',
        displayName: 'AI',
        description: 'AI augmentation, anomalies, and query data',
        iconHint: 'psychology',
        sortOrder: 80,
        category: 'ai',
        actions: [
          PanelAction(name: 'stats'),
          PanelAction(name: 'augmentedEnvelopes', params: [
            PanelActionParam(name: 'limit', type: 'int'),
          ]),
          PanelAction(name: 'aiOriginatedEnvelopes', params: [
            PanelActionParam(name: 'limit', type: 'int'),
          ]),
          PanelAction(name: 'anomalyFlaggedEnvelopes', params: [
            PanelActionParam(name: 'limit', type: 'int'),
          ]),
          PanelAction(name: 'aiProvenanceConfig'),
          PanelAction(name: 'aiQueryConfig'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _provenance != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'stats':
        return stats();
      case 'augmentedEnvelopes':
        return augmentedEnvelopes(limit: params['limit'] as int? ?? 50)
            .map(provenanceEnvelopeToJson)
            .toList();
      case 'aiOriginatedEnvelopes':
        return aiOriginatedEnvelopes(limit: params['limit'] as int? ?? 50)
            .map(provenanceEnvelopeToJson)
            .toList();
      case 'anomalyFlaggedEnvelopes':
        return anomalyFlaggedEnvelopes(limit: params['limit'] as int? ?? 50)
            .map(provenanceEnvelopeToJson)
            .toList();
      case 'aiProvenanceConfig':
        return aiProvenanceConfig();
      case 'aiQueryConfig':
        return aiQueryConfig();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown ai action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns AI-augmented provenance envelopes.
  List<ProvenanceEnvelope> augmentedEnvelopes({int limit = 50}) {
    final all = _provenance!.query();
    final augmented = all.where((e) => e.aiAugmented).toList();
    if (augmented.length <= limit) return augmented;
    return augmented.sublist(augmented.length - limit);
  }

  /// Returns AI-originated provenance envelopes (from AI query fallback).
  List<ProvenanceEnvelope> aiOriginatedEnvelopes({int limit = 50}) {
    final all = _provenance!.query();
    final originated = all.where((e) => e.aiOriginated).toList();
    if (originated.length <= limit) return originated;
    return originated.sublist(originated.length - limit);
  }

  /// Returns envelopes with anomaly flags.
  List<ProvenanceEnvelope> anomalyFlaggedEnvelopes({int limit = 50}) {
    final all = _provenance!.query();
    final flagged = all.where((e) => e.aiAnomalyFlagged).toList();
    if (flagged.length <= limit) return flagged;
    return flagged.sublist(flagged.length - limit);
  }

  /// Returns AI provenance engine configuration.
  Map<String, dynamic> aiProvenanceConfig() {
    if (_aiProvenance == null) return {'enabled': false};
    return {'enabled': true, ..._aiProvenance!.getConfig()};
  }

  /// Returns AI query engine configuration.
  Map<String, dynamic> aiQueryConfig() {
    if (_aiQuery == null) return {'enabled': false};
    return {'enabled': true, ..._aiQuery!.getConfig()};
  }

  /// AI statistics breakdown.
  Map<String, dynamic> stats() {
    final all = _provenance!.query();
    int augmentedCount = 0;
    int originatedCount = 0;
    int anomalyCount = 0;
    final severityCounts = <String, int>{};

    for (final env in all) {
      if (env.aiAugmented) augmentedCount++;
      if (env.aiOriginated) originatedCount++;
      if (env.aiAnomalyFlagged) {
        anomalyCount++;
        final sev = env.aiAnomalySeverity ?? 'unknown';
        severityCounts[sev] = (severityCounts[sev] ?? 0) + 1;
      }
    }

    return {
      'totalEnvelopes': all.length,
      'aiAugmented': augmentedCount,
      'aiOriginated': originatedCount,
      'anomalyFlagged': anomalyCount,
      'anomalySeverity': severityCounts,
    };
  }

  /// Summary for snapshot aggregation.
  @override
  Map<String, dynamic> summary() {
    final all = _provenance!.query();
    return {
      'aiAugmented': all.where((e) => e.aiAugmented).length,
      'aiOriginated': all.where((e) => e.aiOriginated).length,
      'anomalyFlagged': all.where((e) => e.aiAnomalyFlagged).length,
      'aiProvenanceEnabled': _aiProvenance != null,
      'aiQueryEnabled': _aiQuery != null,
    };
  }
}
