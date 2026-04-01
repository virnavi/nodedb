import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

/// Inspector panel for trigger system data.
class TriggerPanel implements InspectorPanel {
  final NoSqlEngine _engine;

  TriggerPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'triggers',
        displayName: 'Triggers',
        description: 'Trigger registry and status',
        iconHint: 'flash_on',
        sortOrder: 60,
        category: 'system',
        actions: [
          PanelAction(name: 'listTriggers'),
          PanelAction(name: 'triggerCount'),
          PanelAction(name: 'triggersByCollection'),
          PanelAction(name: 'enabledTriggers'),
          PanelAction(name: 'disabledTriggers'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'listTriggers':
        return listTriggers();
      case 'triggerCount':
        return triggerCount();
      case 'triggersByCollection':
        return triggersByCollection();
      case 'enabledTriggers':
        return enabledTriggers();
      case 'disabledTriggers':
        return disabledTriggers();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown triggers action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns all registered triggers.
  List<Map<String, dynamic>> listTriggers() => _engine.listTriggers();

  /// Returns the total number of registered triggers.
  int triggerCount() => _engine.listTriggers().length;

  /// Returns triggers grouped by collection name.
  Map<String, List<Map<String, dynamic>>> triggersByCollection() {
    final triggers = _engine.listTriggers();
    final grouped = <String, List<Map<String, dynamic>>>{};
    for (final t in triggers) {
      final col = t['collection']?.toString() ?? 'unknown';
      grouped.putIfAbsent(col, () => []).add(t);
    }
    return grouped;
  }

  /// Returns only enabled triggers.
  List<Map<String, dynamic>> enabledTriggers() =>
      _engine.listTriggers().where((t) => t['enabled'] == true).toList();

  /// Returns only disabled triggers.
  List<Map<String, dynamic>> disabledTriggers() =>
      _engine.listTriggers().where((t) => t['enabled'] != true).toList();

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    final triggers = _engine.listTriggers();
    return {
      'totalTriggers': triggers.length,
      'enabled': triggers.where((t) => t['enabled'] == true).length,
      'disabled': triggers.where((t) => t['enabled'] != true).length,
      'triggers': triggers,
    };
  }
}
