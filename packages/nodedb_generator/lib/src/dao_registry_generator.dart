import 'model_info.dart';

/// Generates a DAO registry class and NodeDB extension.
///
/// Given a list of models, produces:
/// - `NodeDbDaos` class with a DAO field per collection
/// - `NodeDbDaoAccess` extension on `NodeDB` for `db.dao.users`
///
/// This is a top-level generator called once per build, not per-model.
/// For now, each model's generated file includes instructions for manual
/// registration since build_runner generates per-file.
String generateDaoRegistry(List<ModelInfo> models) {
  final buffer = StringBuffer();

  buffer.writeln(
      '// ── DAO Registry ───────────────────────────────────────────────');

  // DAO registry class
  buffer.writeln('class NodeDbDaos {');

  // Fields
  for (final model in models) {
    final daoType = _daoTypeName(model);
    final fieldName = _fieldName(model);
    buffer.writeln('  final $daoType $fieldName;');
  }

  // Constructor
  buffer.write('  NodeDbDaos(');
  final hasCollections = models.any((m) => m.type == ModelType.collection || m.type == ModelType.view);
  final hasGraph = models.any((m) => m.type == ModelType.node || m.type == ModelType.edge);
  if (hasCollections) {
    buffer.write('NoSqlEngine nosqlEngine');
  }
  if (hasGraph) {
    if (hasCollections) buffer.write(', ');
    buffer.write('GraphEngine graphEngine');
  }
  buffer.write(', {ProvenanceEngine? provenanceEngine, CollectionNotifier? notifier, String? databaseName, TransportEngine? transport}');
  buffer.writeln(')');
  buffer.write('    : ');
  final inits = <String>[];
  for (final model in models) {
    final fieldName = _fieldName(model);
    final daoType = _daoTypeName(model);
    if (model.type == ModelType.collection) {
      inits.add('$fieldName = $daoType(nosqlEngine, provenanceEngine, notifier, databaseName)');
    } else if (model.type == ModelType.view) {
      inits.add('$fieldName = $daoType(nosqlEngine, transport, databaseName)');
    } else {
      inits.add('$fieldName = $daoType(graphEngine)');
    }
  }
  buffer.writeln(inits.join(',\n      '));
  buffer.writeln('  ;');

  buffer.writeln('}');

  return buffer.toString();
}

/// Generates the per-model concrete DAO class stub that extends the base DAO.
///
/// This is a one-time scaffold — users add custom query methods here.
String generateConcreteDao(ModelInfo model) {
  final baseName = _baseDaoName(model);
  final concreteName = _daoTypeName(model);

  if (model.type == ModelType.collection) {
    return '''
// ── Concrete DAO ───────────────────────────────────────────────
class $concreteName extends $baseName {
  @override
  final NoSqlEngine engine;
  @override
  final ProvenanceEngine? provenanceEngine;
  @override
  final CollectionNotifier? notifier;
  @override
  final String? databaseName;
  $concreteName(this.engine, [this.provenanceEngine, this.notifier, this.databaseName]);
  // Add custom query methods here
}
''';
  } else if (model.type == ModelType.node) {
    return '''
class $concreteName extends $baseName {
  @override
  final GraphEngine graphEngine;
  $concreteName(this.graphEngine);
}
''';
  } else {
    return '''
class $concreteName extends $baseName {
  @override
  final GraphEngine graphEngine;
  $concreteName(this.graphEngine);
}
''';
  }
}

/// Generates an extension on NodeDB for typed DAO access.
///
/// Produces `db.dao.users`, `db.dao.articles` etc.
String generateDaoExtension(List<ModelInfo> models) {
  final buffer = StringBuffer();

  buffer.writeln('// ── NodeDB DAO Extension ────────────────────────────────────────');
  buffer.writeln('extension NodeDbDaoAccess on NodeDB {');
  buffer.writeln('  NodeDbDaos get dao {');

  // Build constructor args
  buffer.write('    return NodeDbDaos(');
  final args = <String>[];
  if (models.any((m) => m.type == ModelType.collection || m.type == ModelType.view)) {
    args.add('nosql');
  }
  if (models.any((m) => m.type == ModelType.node || m.type == ModelType.edge)) {
    args.add('graph!');
  }
  buffer.write(args.join(', '));
  buffer.writeln(', provenanceEngine: provenance, notifier: notifier, databaseName: databaseName, transport: transport);');

  buffer.writeln('  }');
  buffer.writeln('}');

  return buffer.toString();
}

String _daoTypeName(ModelInfo model) => '${model.className}Dao';

String _baseDaoName(ModelInfo model) {
  switch (model.type) {
    case ModelType.collection:
      return '${model.className}DaoBase';
    case ModelType.node:
      return '${model.className}NodeDaoBase';
    case ModelType.edge:
      return '${model.className}EdgeDaoBase';
    case ModelType.preferences:
      return '${model.className}PrefsBase';
    case ModelType.view:
      return '${model.className}ViewDaoBase';
  }
}

String _fieldName(ModelInfo model) {
  final name = model.collectionName;
  // camelCase: already lowercase/snake, just return as-is
  return name;
}
