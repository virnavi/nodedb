import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

/// Inspector panel for preference stores.
class PreferencePanel implements InspectorPanel {
  final NoSqlEngine _engine;

  PreferencePanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'preferences',
        displayName: 'Prefs',
        description: 'Preference key-value stores',
        iconHint: 'settings_applications',
        sortOrder: 70,
        category: 'system',
        actions: [
          PanelAction(name: 'keys', params: [
            PanelActionParam(name: 'store', type: 'string', required: true),
          ]),
          PanelAction(name: 'getValue', params: [
            PanelActionParam(name: 'store', type: 'string', required: true),
            PanelActionParam(name: 'key', type: 'string', required: true),
          ]),
          PanelAction(name: 'allValues', params: [
            PanelActionParam(name: 'store', type: 'string', required: true),
          ]),
          PanelAction(name: 'shareableEntries', params: [
            PanelActionParam(name: 'store', type: 'string', required: true),
          ]),
          PanelAction(name: 'storeSummary', params: [
            PanelActionParam(name: 'store', type: 'string', required: true),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'keys':
        return keys(params['store'] as String);
      case 'getValue':
        return getValue(params['store'] as String, params['key'] as String);
      case 'allValues':
        return allValues(params['store'] as String);
      case 'shareableEntries':
        return shareableEntries(params['store'] as String);
      case 'storeSummary':
        return storeSummary(params['store'] as String);
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown preferences action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns all keys in a preference store.
  List<String> keys(String store) => _engine.prefKeys(store);

  /// Returns a preference value by key.
  dynamic getValue(String store, String key) => _engine.prefGet(store, key);

  /// Returns all key-value pairs in a preference store.
  Map<String, dynamic> allValues(String store) {
    final ks = _engine.prefKeys(store);
    final result = <String, dynamic>{};
    for (final k in ks) {
      final resp = _engine.prefGet(store, k);
      if (resp is Map && resp['found'] == true) {
        result[k] = resp['value'];
      }
    }
    return result;
  }

  /// Returns shareable preference entries.
  List<Map<String, dynamic>> shareableEntries(String store) =>
      _engine.prefShareable(store);

  /// Returns summary for a given store.
  Map<String, dynamic> storeSummary(String store) {
    final ks = _engine.prefKeys(store);
    return {
      'store': store,
      'keyCount': ks.length,
      'keys': ks,
    };
  }

  @override
  Map<String, dynamic> summary() => {'available': true};
}
