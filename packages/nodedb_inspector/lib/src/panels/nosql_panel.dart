import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import '../server/json_serializers.dart';

/// Inspector panel for NoSQL engine data.
class NoSqlPanel implements InspectorPanel {
  final NoSqlEngine _engine;

  NoSqlPanel(this._engine);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'nosql',
        displayName: 'NoSQL',
        description: 'Document store collections and documents',
        iconHint: 'storage',
        sortOrder: 10,
        category: 'data',
        actions: [
          PanelAction(name: 'collectionStats'),
          PanelAction(name: 'collectionNames'),
          PanelAction(name: 'documentPreview', params: [
            PanelActionParam(name: 'collection', type: 'string', required: true),
            PanelActionParam(name: 'limit', type: 'int'),
            PanelActionParam(name: 'offset', type: 'int'),
          ]),
          PanelAction(name: 'documentDetail', params: [
            PanelActionParam(name: 'collection', type: 'string', required: true),
            PanelActionParam(name: 'id', type: 'int', required: true),
          ]),
          PanelAction(name: 'schemaList'),
          PanelAction(name: 'schemaFingerprint'),
          PanelAction(name: 'totalDocuments'),
          PanelAction(name: 'summary'),
        ],
      );

  @override
  bool get isAvailable => true;

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'collectionStats':
        return collectionStats();
      case 'collectionNames':
        return collectionNames();
      case 'documentPreview':
        return documentPreview(
          params['collection'] as String,
          limit: params['limit'] as int? ?? 20,
          offset: params['offset'] as int? ?? 0,
        ).map(documentToJson).toList();
      case 'documentDetail':
        final doc = documentDetail(params['collection'] as String, params['id'] as int);
        return doc != null ? documentToJson(doc) : null;
      case 'schemaList':
        return schemaList();
      case 'schemaFingerprint':
        return schemaFingerprint();
      case 'totalDocuments':
        return totalDocuments();
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown nosql action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns document count per collection.
  Map<String, int> collectionStats() {
    final names = _engine.collectionNames();
    final stats = <String, int>{};
    for (final name in names) {
      // Strip schema prefix for count (e.g. "public.users" → "users")
      final bare = name.contains('.') ? name.split('.').last : name;
      stats[name] = _engine.count(bare);
    }
    return stats;
  }

  /// Returns all collection names (schema-qualified).
  List<String> collectionNames() => _engine.collectionNames();

  /// Returns a preview of documents in a collection.
  List<Document> documentPreview(String collection, {int limit = 20, int offset = 0}) {
    return _engine.findAll(collection, limit: limit, offset: offset);
  }

  /// Returns a single document by ID.
  Document? documentDetail(String collection, int id) {
    return _engine.get(collection, id);
  }

  /// Returns all schemas.
  List<Map<String, dynamic>> schemaList() => _engine.listSchemas();

  /// Returns the schema fingerprint.
  String schemaFingerprint() => _engine.schemaFingerprint();

  /// Returns the total document count across all collections.
  int totalDocuments() {
    return collectionStats().values.fold(0, (sum, count) => sum + count);
  }

  /// Summary data for the snapshot.
  @override
  Map<String, dynamic> summary() {
    final stats = collectionStats();
    return {
      'collections': stats,
      'totalDocuments': stats.values.fold(0, (int sum, count) => sum + count),
      'schemaFingerprint': schemaFingerprint(),
    };
  }
}
