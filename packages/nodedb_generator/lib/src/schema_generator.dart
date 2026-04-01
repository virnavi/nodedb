import 'model_info.dart';

/// Generates a `NodeDbSchema` constant for a model.
String generateSchema(ModelInfo model) {
  final buffer = StringBuffer();

  buffer.writeln('// ── Schema ─────────────────────────────────────────────────────');
  buffer.writeln(
      'const _${_camelCase(model.className)}Schema = NodeDbSchema(');
  buffer.writeln("  name: '${model.collectionName}',");
  buffer.writeln("  schema: '${model.schema ?? 'public'}',");
  buffer.writeln('  singleton: ${model.singleton},');
  buffer.writeln('  type: ${_modelTypeString(model.type)},');
  buffer.writeln('  fields: [');

  for (final field in model.fields) {
    final fieldType = _fieldTypeString(field);
    buffer.write("    SchemaField('${field.name}', $fieldType");
    if (field.name == 'id' && field.isString) {
      buffer.write(', indexed: true, unique: true');
    } else if (field.index != null) {
      buffer.write(', indexed: true');
      if (field.index!.unique) buffer.write(', unique: true');
    }
    buffer.writeln('),');
  }

  buffer.writeln('  ],');
  if (model.trimmable) {
    buffer.writeln('  trimmable: true,');
    if (model.trimPolicy != null) {
      buffer.writeln("  trimPolicy: '${model.trimPolicy}',");
    }
  }
  if (model.neverTrim) {
    buffer.writeln('  neverTrim: true,');
  }
  if (!model.backup) {
    buffer.writeln('  backup: false,');
  }
  buffer.writeln(');');

  // Schema JSON constant for AI adapter consumption
  buffer.writeln();
  buffer.writeln('const _${_camelCase(model.className)}SchemaJson = <String, dynamic>{');
  buffer.writeln("  'name': '${model.collectionName}',");
  buffer.writeln("  'schema': '${model.schema ?? 'public'}',");
  buffer.writeln("  'type': ${_modelTypeString(model.type)},");
  buffer.writeln("  'required_fields': [");
  for (final field in model.fields) {
    if (!field.isNullable && field.name != 'id') {
      buffer.writeln("    '${field.name}',");
    }
  }
  buffer.writeln('  ],');
  buffer.writeln("  'field_types': {");
  for (final field in model.fields) {
    buffer.writeln("    '${field.name}': ${_fieldTypeString(field)},");
  }
  buffer.writeln('  },');
  buffer.writeln('};');

  // FQN constant for easy reference
  final schemaPrefix = model.schema ?? 'public';
  final fqnVarName = _camelCase(model.className);
  buffer.writeln();
  buffer.writeln('/// Fully qualified collection name: $schemaPrefix.${model.collectionName}');
  buffer.writeln("const ${fqnVarName}CollectionName = '$schemaPrefix.${model.collectionName}';");

  return buffer.toString();
}

/// Schema type constants used in generated code.
///
/// These are lightweight metadata objects, not imported from Rust.
String generateSchemaTypes() {
  return '''
/// Schema metadata for a NodeDB collection/node/edge.
class NodeDbSchema {
  final String name;
  final String schema;
  final bool singleton;
  final String type;
  final List<SchemaField> fields;
  final bool trimmable;
  final String? trimPolicy;
  final bool neverTrim;
  final bool backup;

  const NodeDbSchema({
    required this.name,
    required this.schema,
    required this.singleton,
    required this.type,
    required this.fields,
    this.trimmable = false,
    this.trimPolicy,
    this.neverTrim = false,
    this.backup = true,
  });

  String get qualifiedName => '\$schema.\$name';
}

/// Field metadata within a schema.
class SchemaField {
  final String name;
  final String type;
  final bool indexed;
  final bool unique;

  const SchemaField(this.name, this.type, {this.indexed = false, this.unique = false});
}
''';
}

String _camelCase(String name) {
  return '${name[0].toLowerCase()}${name.substring(1)}';
}

String _modelTypeString(ModelType type) {
  switch (type) {
    case ModelType.collection:
      return "'collection'";
    case ModelType.node:
      return "'node'";
    case ModelType.edge:
      return "'edge'";
    case ModelType.preferences:
      return "'preferences'";
    case ModelType.view:
      return "'view'";
  }
}

String _fieldTypeString(FieldInfo field) {
  if (field.dartType == 'double') return "'double'";
  if (field.dartType == 'num') return "'num'";
  if (field.dartType == 'int') return "'int'";
  if (field.isString) return "'string'";
  if (field.isBool) return "'bool'";
  if (field.isDateTime) return "'datetime'";
  if (field.isList) return "'list'";
  if (field.isEmbedded) return "'embedded'";
  if (field.isEnumerated) return "'enum'";
  if (field.isJsonbLike || field.isMap) return "'jsonb'";
  return "'dynamic'";
}
