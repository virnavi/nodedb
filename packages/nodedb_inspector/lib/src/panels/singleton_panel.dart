import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for singleton collections.
class SingletonPanel implements InspectorPanel {
  final NoSqlEngine _engine;

  SingletonPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'singletons',
        displayName: 'Singletons',
        description: 'Singleton collections',
        iconHint: 'tune',
        sortOrder: 65,
        category: 'system',
        actions: [
          PanelAction(name: 'singletonNames'),
          PanelAction(name: 'singletonPreview'),
          PanelAction(name: 'singletonData', params: [
            PanelActionParam(
                name: 'collection', type: 'string', required: true),
          ]),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'singletonNames':
        return singletonNames();
      case 'singletonPreview':
        return singletonPreview();
      case 'singletonData':
        return documentToJson(singletonData(params['collection'] as String));
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown singletons action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns names of all singleton collections.
  List<String> singletonNames() {
    final names = _engine.collectionNames();
    return names.where((name) {
      final bare = name.contains('.') ? name.split('.').last : name;
      return _engine.isSingleton(bare);
    }).toList();
  }

  /// Returns the singleton document for a collection.
  Document singletonData(String collection) => _engine.singletonGet(collection);

  /// Returns a preview of all singletons with their current data.
  List<Map<String, dynamic>> singletonPreview() {
    final names = singletonNames();
    return names.map((name) {
      final bare = name.contains('.') ? name.split('.').last : name;
      try {
        final doc = _engine.singletonGet(bare);
        return <String, dynamic>{
          'collection': name,
          'id': doc.id,
          'data': doc.data,
        };
      } catch (_) {
        return <String, dynamic>{
          'collection': name,
          'error': 'failed to read',
        };
      }
    }).toList();
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    final names = singletonNames();
    return {
      'count': names.length,
      'collections': names,
    };
  }
}
