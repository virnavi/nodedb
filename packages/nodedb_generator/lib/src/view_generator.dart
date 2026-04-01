import 'model_info.dart';

/// Generates a read-only ViewDao for a `@NodeDBView` model.
///
/// Views query multiple source collections and merge results.
/// No write methods are generated.
String generateViewDao(ModelInfo model) {
  final cls = model.className;
  final daoName = '${cls}ViewDaoBase';
  final viewInfo = model.viewInfo!;

  var buf = '''
// ── View DAO ───────────────────────────────────────────────────
abstract class $daoName {
  NoSqlEngine get engine;
  TransportEngine? get transport => null;
  String? get databaseName => null;

  static const schemaName = '${model.schema ?? 'public'}';
  String get collectionName => '${model.collectionName}';
  String get qualifiedName {
    final db = databaseName;
    if (db != null && db.isNotEmpty) return '\$db.\$schemaName.\$collectionName';
    return '\$schemaName.\$collectionName';
  }

  static const sources = <Map<String, String>>[
''';

  for (final source in viewInfo.sources) {
    buf += "    {'collection': '${source.collection}', 'schema': '${source.schema}'";
    if (source.database != null) {
      buf += ", 'database': '${source.database}'";
    }
    buf += '},\n';
  }

  buf += '''
  ];

  $cls _fromMap(Map<String, dynamic> map) => _\$${cls}FromMap(map);

  List<$cls> findAll([FilterQuery<$cls>? query]) {
    final params = query?.build() ?? {};
    final filter = params['filter'] as Map<String, dynamic>?;
    final sort = (params['sort'] as List?)?.cast<Map<String, dynamic>>();
    final offset = params['offset'] as int?;
    final limit = params['limit'] as int?;

    final results = <$cls>[];

    for (final source in sources) {
      final collection = source['collection']!;
      final database = source['database'];

      if (database != null && database.isNotEmpty && transport != null) {
        // Remote source — query via mesh
        final remoteDocs = transport!.meshQuery(
          database: database,
          queryType: 'find_all',
          queryData: {
            'collection': collection,
            if (filter != null) 'filter': filter,
            if (sort != null) 'sort': sort,
          },
        );
        results.addAll(remoteDocs.map((doc) => _fromMap(doc.data)));
      } else {
        // Local source
        final docs = engine.findAll(
          collection,
          filter: filter,
          sort: sort,
        );
        results.addAll(docs.map((doc) => _fromMap(doc.data)));
      }
    }

    // Apply offset/limit after merge
    var merged = results;
    if (offset != null && offset > 0) {
      merged = merged.skip(offset).toList();
    }
    if (limit != null && limit > 0) {
      merged = merged.take(limit).toList();
    }

    return merged;
  }

  $cls? findFirst(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    final query = filter(FilterQuery<$cls>())..limit(1);
    final results = findAll(query);
    return results.isEmpty ? null : results.first;
  }

  List<$cls> findWhere(FilterQuery<$cls> Function(FilterQuery<$cls>) filter) {
    return findAll(filter(FilterQuery<$cls>()));
  }

  int count() {
    var total = 0;
    for (final source in sources) {
      final collection = source['collection']!;
      final database = source['database'];
      if (database != null && database.isNotEmpty && transport != null) {
        final result = transport!.meshQuery(
          database: database,
          queryType: 'count',
          queryData: {'collection': collection},
        );
        total += (result is int ? result : (result as List).length);
      } else {
        total += engine.count(collection);
      }
    }
    return total;
  }
}
''';

  return buf;
}

/// Generates the concrete ViewDao class.
String generateConcreteViewDao(ModelInfo model) {
  final cls = model.className;
  final baseName = '${cls}ViewDaoBase';
  final concreteName = '${cls}Dao';

  return '''
// ── Concrete View DAO ──────────────────────────────────────────
class $concreteName extends $baseName {
  @override
  final NoSqlEngine engine;
  @override
  final TransportEngine? transport;
  @override
  final String? databaseName;
  $concreteName(this.engine, [this.transport, this.databaseName]);
}
''';
}
