import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

/// Inspector panel for schema overview.
class SchemaPanel implements InspectorPanel {
  final NoSqlEngine _nosql;
  final ProvenanceEngine? _provenance;

  SchemaPanel(this._nosql, this._provenance);

  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
        id: 'schema',
        displayName: 'Schema',
        description: 'Schema metadata and collection info',
        iconHint: 'schema',
        sortOrder: 15,
        category: 'data',
        actions: [
          PanelAction(name: 'overview'),
          PanelAction(name: 'collectionDetail', params: [
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
      case 'overview':
        return overview();
      case 'collectionDetail':
        return collectionDetail(params['collection'] as String);
      case 'summary':
        return summary();
      default:
        throw ArgumentError('Unknown schema action: $action');
    }
  }

  @override
  void onRegister() {}

  @override
  void onUnregister() {}

  /// Returns a schema overview: schemas, collections, fingerprint.
  Map<String, dynamic> overview() {
    return {
      'schemas': _nosql.listSchemas(),
      'collections': _nosql.collectionNames(),
      'fingerprint': _nosql.schemaFingerprint(),
    };
  }

  /// Returns detail for a specific collection: schema info + counts.
  Map<String, dynamic> collectionDetail(String collection) {
    final result = <String, dynamic>{
      'name': collection,
      'documentCount': _nosql.count(collection),
    };

    final prov = _provenance;
    if (prov != null) {
      final envelopes = prov.query(collection: collection);
      result['provenanceCount'] = envelopes.length;
    }

    return result;
  }

  @override
  Map<String, dynamic> summary() => overview();
}
