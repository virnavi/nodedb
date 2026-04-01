import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

/// Inspector panel for access history data.
class AccessHistoryPanel implements InspectorPanel {
  final NoSqlEngine _engine;

  AccessHistoryPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'accessHistory',
        displayName: 'History',
        description: 'Access history audit logs',
        iconHint: 'history',
        sortOrder: 75,
        category: 'system',
        actions: [
          PanelAction(name: 'query', params: [
            PanelActionParam(name: 'collection', type: 'string'),
            PanelActionParam(name: 'recordId', type: 'int'),
            PanelActionParam(name: 'eventType', type: 'string'),
          ]),
          PanelAction(name: 'count'),
          PanelAction(name: 'lastAccess', params: [
            PanelActionParam(
                name: 'collection', type: 'string', required: true),
            PanelActionParam(name: 'recordId', type: 'int', required: true),
          ]),
          PanelAction(name: 'heatmap'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'query':
        return query(
          collection: params['collection'] as String?,
          recordId: params['recordId'] as int?,
          eventType: params['eventType'] as String?,
        );
      case 'count':
        return count();
      case 'lastAccess':
        return lastAccess(
          params['collection'] as String,
          params['recordId'] as int,
        );
      case 'heatmap':
        return heatmap();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown accessHistory action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Query access history entries with optional filters.
  List<Map<String, dynamic>> query({
    String? collection,
    int? recordId,
    String? eventType,
  }) =>
      _engine.accessHistoryQuery(
        collection: collection,
        recordId: recordId,
        eventType: eventType,
      );

  /// Total access history entry count.
  int count() => _engine.accessHistoryCount();

  /// Last access time for a specific record.
  String? lastAccess(String collection, int recordId) =>
      _engine.accessHistoryLastAccess(collection, recordId);

  /// Trim old entries beyond retention period.
  int trim({int retentionSecs = 365 * 24 * 3600}) =>
      _engine.accessHistoryTrim(retentionSecs: retentionSecs);

  /// Heatmap data: count of accesses per collection.
  Map<String, int> heatmap() {
    final entries = _engine.accessHistoryQuery();
    final counts = <String, int>{};
    for (final entry in entries) {
      final col = entry['collection']?.toString() ?? 'unknown';
      counts[col] = (counts[col] ?? 0) + 1;
    }
    return counts;
  }

  /// Summary for snapshot aggregation.
  @override
  Map<String, dynamic> summary() => {
        'totalEntries': count(),
        'heatmap': heatmap(),
      };
}
