/// Parsed representation of an annotated NodeDB model class.
///
/// Built by the annotation parser, consumed by all sub-generators.
library;

enum ModelType { collection, node, edge, preferences, view }

class ModelInfo {
  final String className;
  final String collectionName;
  final ModelType type;
  final String? schema;
  final bool singleton;
  final bool generateDao;
  final List<FieldInfo> fields;
  final List<IndexInfo> indexes;
  final EdgeInfo? edgeInfo;
  final bool trimmable;
  final bool neverTrim;
  final String? trimPolicy;
  final bool backup;
  final List<TriggerInfo> triggers;
  final ViewInfo? viewInfo;

  const ModelInfo({
    required this.className,
    required this.collectionName,
    required this.type,
    this.schema,
    this.singleton = false,
    this.generateDao = true,
    required this.fields,
    required this.indexes,
    this.edgeInfo,
    this.trimmable = false,
    this.neverTrim = false,
    this.trimPolicy,
    this.backup = true,
    this.triggers = const [],
    this.viewInfo,
  });

  /// Fields excluding 'id' (used for serialization).
  List<FieldInfo> get dataFields => fields.where((f) => f.name != 'id').toList();

  /// The id field, if present.
  FieldInfo? get idField {
    try {
      return fields.firstWhere((f) => f.name == 'id');
    } catch (_) {
      return null;
    }
  }

  /// Whether this model uses a String-type ID (UUID mode).
  bool get isStringId => idField != null && idField!.dartType == 'String';

  /// Whether this model uses an int-type ID (legacy sled key mode).
  bool get isIntId => idField == null || idField!.dartType == 'int';
}

class FieldInfo {
  final String name;
  final String dartType;
  final bool isNullable;
  final bool isEmbedded;
  final bool isEnumerated;
  final bool isJsonb;
  final bool isJsonModel;
  final String? jsonModelType;
  final String? jsonbSchema;
  final String? jsonbIdentifier;
  final bool isList;
  final String? listElementType;
  final IndexInfo? index;
  final VectorFieldInfo? vectorField;

  const FieldInfo({
    required this.name,
    required this.dartType,
    this.isNullable = false,
    this.isEmbedded = false,
    this.isEnumerated = false,
    this.isJsonb = false,
    this.isJsonModel = false,
    this.jsonModelType,
    this.jsonbSchema,
    this.jsonbIdentifier,
    this.isList = false,
    this.listElementType,
    this.index,
    this.vectorField,
  });

  /// Whether this field is JSONB-like (explicit @Jsonb or @JsonModel typed).
  bool get isJsonbLike => isJsonb || isJsonModel;

  /// Whether this field is a numeric type.
  bool get isNumeric =>
      dartType == 'int' || dartType == 'double' || dartType == 'num';

  /// Whether this field is a string type.
  bool get isString => dartType == 'String';

  /// Whether this field is a bool type.
  bool get isBool => dartType == 'bool';

  /// Whether this field is a DateTime type.
  bool get isDateTime => dartType == 'DateTime';

  /// Whether this field is a Map type (JSONB-like).
  bool get isMap => dartType == 'Map<String, dynamic>';
}

class IndexInfo {
  final String type;
  final bool unique;
  final List<String> composite;

  const IndexInfo({
    this.type = 'value',
    this.unique = false,
    this.composite = const [],
  });
}

class VectorFieldInfo {
  final int dimensions;
  final String metric;

  const VectorFieldInfo({required this.dimensions, this.metric = 'cosine'});
}

class EdgeInfo {
  final String fromType;
  final String toType;

  const EdgeInfo({required this.fromType, required this.toType});
}

class TriggerInfo {
  final String event;
  final String timing;
  final String? name;

  const TriggerInfo({
    required this.event,
    required this.timing,
    this.name,
  });
}

class ViewInfo {
  final List<ViewSourceInfo> sources;
  final String strategy;
  final String? joinField;

  const ViewInfo({
    required this.sources,
    this.strategy = 'union',
    this.joinField,
  });
}

class ViewSourceInfo {
  final String collection;
  final String schema;
  final String? database;

  const ViewSourceInfo({
    required this.collection,
    this.schema = 'public',
    this.database,
  });
}

/// Convert a PascalCase class name to snake_case collection name.
String toSnakeCase(String name) {
  return name
      .replaceAllMapped(RegExp('([A-Z])'), (m) => '_${m[1]!.toLowerCase()}')
      .replaceFirst('_', '');
}

/// Pluralize a snake_case name (simple English rules).
String pluralize(String name) {
  if (name.endsWith('s') || name.endsWith('sh') || name.endsWith('ch') || name.endsWith('x')) {
    return '${name}es';
  }
  if (name.endsWith('y') && !name.endsWith('ay') && !name.endsWith('ey') && !name.endsWith('oy') && !name.endsWith('uy')) {
    return '${name.substring(0, name.length - 1)}ies';
  }
  return '${name}s';
}
