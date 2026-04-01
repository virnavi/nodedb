import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for Provenance engine data.
class ProvenancePanel implements InspectorPanel {
  final ProvenanceEngine? _engine;

  ProvenancePanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'provenance',
        displayName: 'Provenance',
        description: 'Provenance envelopes and confidence',
        iconHint: 'verified',
        sortOrder: 45,
        category: 'security',
        actions: [
          PanelAction(name: 'stats'),
          PanelAction(name: 'confidenceHistogram', params: [
            PanelActionParam(name: 'collection', type: 'string', required: true),
          ]),
          PanelAction(name: 'envelopesForRecord', params: [
            PanelActionParam(name: 'collection', type: 'string', required: true),
            PanelActionParam(name: 'recordId', type: 'int', required: true),
          ]),
          PanelAction(name: 'recentEnvelopes', params: [
            PanelActionParam(name: 'limit', type: 'int'),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _engine != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'stats':
        return stats();
      case 'confidenceHistogram':
        return confidenceHistogram(params['collection'] as String);
      case 'envelopesForRecord':
        return envelopesForRecord(
          params['collection'] as String,
          params['recordId'] as int,
        ).map(provenanceEnvelopeToJson).toList();
      case 'recentEnvelopes':
        return recentEnvelopes(limit: params['limit'] as int? ?? 50)
            .map(provenanceEnvelopeToJson)
            .toList();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown provenance action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns provenance statistics.
  Map<String, dynamic> stats() {
    final all = _engine!.query();
    final sourceTypes = <String, int>{};
    final verificationStatuses = <String, int>{};

    for (final env in all) {
      sourceTypes[env.sourceType] = (sourceTypes[env.sourceType] ?? 0) + 1;
      verificationStatuses[env.verificationStatus] =
          (verificationStatuses[env.verificationStatus] ?? 0) + 1;
    }

    return {
      'totalCount': _engine!.count(),
      'sourceTypeBreakdown': sourceTypes,
      'verificationBreakdown': verificationStatuses,
    };
  }

  /// Returns a confidence histogram for a collection.
  ///
  /// Buckets: 0.0-0.2, 0.2-0.4, 0.4-0.6, 0.6-0.8, 0.8-1.0
  Map<String, int> confidenceHistogram(String collection) {
    final envelopes = _engine!.query(collection: collection);
    final buckets = <String, int>{
      '0.0-0.2': 0,
      '0.2-0.4': 0,
      '0.4-0.6': 0,
      '0.6-0.8': 0,
      '0.8-1.0': 0,
    };

    for (final env in envelopes) {
      final c = env.confidenceFactor;
      if (c < 0.2) {
        buckets['0.0-0.2'] = buckets['0.0-0.2']! + 1;
      } else if (c < 0.4) {
        buckets['0.2-0.4'] = buckets['0.2-0.4']! + 1;
      } else if (c < 0.6) {
        buckets['0.4-0.6'] = buckets['0.4-0.6']! + 1;
      } else if (c < 0.8) {
        buckets['0.6-0.8'] = buckets['0.6-0.8']! + 1;
      } else {
        buckets['0.8-1.0'] = buckets['0.8-1.0']! + 1;
      }
    }

    return buckets;
  }

  /// Returns provenance envelopes for a specific record.
  List<ProvenanceEnvelope> envelopesForRecord(String collection, int recordId) {
    return _engine!.getForRecord(collection, recordId);
  }

  /// Returns recent provenance envelopes.
  List<ProvenanceEnvelope> recentEnvelopes({int limit = 50}) {
    final all = _engine!.query();
    if (all.length <= limit) return all;
    return all.sublist(all.length - limit);
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    return {
      'envelopeCount': _engine!.count(),
    };
  }
}
