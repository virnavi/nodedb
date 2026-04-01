import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for KeyResolver engine data.
class KeyResolverPanel implements InspectorPanel {
  final KeyResolverEngine? _engine;

  KeyResolverPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'keyResolver',
        displayName: 'Keys',
        description: 'PKI key cache and trust levels',
        iconHint: 'vpn_key',
        sortOrder: 50,
        category: 'security',
        actions: [
          PanelAction(name: 'keyList'),
          PanelAction(name: 'keyStats'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => _engine != null;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'keyList':
        return keyList().map(keyEntryToJson).toList();
      case 'keyStats':
        return keyStats();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown keyResolver action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns all keys in the cache.
  List<KeyEntry> keyList() => _engine!.allKeys();

  /// Returns key statistics.
  Map<String, dynamic> keyStats() {
    final keys = _engine!.allKeys();
    final trustLevels = <String, int>{};
    for (final key in keys) {
      trustLevels[key.trustLevel] = (trustLevels[key.trustLevel] ?? 0) + 1;
    }
    return {
      'totalKeys': _engine!.keyCount(),
      'trustLevelBreakdown': trustLevels,
      'trustAllActive': _engine!.isTrustAllActive(),
    };
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    return {
      'keyCount': _engine!.keyCount(),
      'trustAllActive': _engine!.isTrustAllActive(),
    };
  }
}
