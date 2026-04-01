import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for Vector engine data.
class VectorPanel implements InspectorPanel {
  final VectorEngine? _engine;
  final VectorOpenConfig? _config;

  VectorPanel(this._engine, this._config);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'vector',
        displayName: 'Vector',
        description: 'Vector search engine',
        iconHint: 'scatter_plot',
        sortOrder: 25,
        category: 'data',
        actions: [
          PanelAction(name: 'stats'),
          PanelAction(name: 'search', params: [
            PanelActionParam(name: 'query', type: 'list', required: true),
            PanelActionParam(name: 'k', type: 'int'),
          ]),
          PanelAction(name: 'recordDetail', params: [
            PanelActionParam(name: 'id', type: 'int', required: true),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _engine != null && _config != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'stats':
        return stats();
      case 'search':
        final query = (params['query'] as List).map((v) => (v as num).toDouble()).toList();
        return search(query, k: params['k'] as int? ?? 10)
            .map(searchResultToJson)
            .toList();
      case 'recordDetail':
        final rec = recordDetail(params['id'] as int);
        return rec != null ? vectorRecordToJson(rec) : null;
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown vector action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns vector engine statistics.
  Map<String, dynamic> stats() {
    return {
      'count': _engine!.count(),
      'dimension': _config!.dimension,
      'metric': _config!.metric,
      'maxElements': _config!.maxElements,
    };
  }

  /// Searches for nearest neighbors.
  List<SearchResult> search(List<double> query, {int k = 10}) {
    return _engine!.search(query, k: k);
  }

  /// Returns a single vector record.
  VectorRecord? recordDetail(int id) => _engine!.get(id);

  @override
  Map<String, dynamic> summary() => stats();
}
