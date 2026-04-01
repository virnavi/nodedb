import 'package:analyzer/dart/element/element.dart';
import 'package:analyzer/dart/element/nullability_suffix.dart';
import 'package:analyzer/dart/element/type.dart';
import 'package:build/build.dart';
import 'package:nodedb/nodedb.dart';
import 'package:source_gen/source_gen.dart';

import 'model_info.dart';
import 'schema_generator.dart';
import 'filter_generator.dart';
import 'serialization_generator.dart';
import 'dao_generator.dart';
import 'dao_registry_generator.dart';
import 'preferences_generator.dart';
import 'view_generator.dart';

/// Generator for `@collection` annotated classes.
class CollectionGenerator extends GeneratorForAnnotation<Collection> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@collection can only be applied to classes.',
        element: element,
      );
    }

    final model = _parseModel(element, annotation, ModelType.collection);
    return _generate(model);
  }
}

/// Generator for `@node` annotated classes.
class NodeGenerator extends GeneratorForAnnotation<Node> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@node can only be applied to classes.',
        element: element,
      );
    }

    final model = _parseModel(element, annotation, ModelType.node);
    return _generate(model);
  }
}

/// Generator for `@Edge` annotated classes.
class EdgeGenerator extends GeneratorForAnnotation<Edge> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@Edge can only be applied to classes.',
        element: element,
      );
    }

    final fromType = annotation.read('from').typeValue;
    final toType = annotation.read('to').typeValue;

    final model = _parseModel(element, annotation, ModelType.edge,
        edgeInfo: EdgeInfo(
          fromType: fromType.getDisplayString(withNullability: true),
          toType: toType.getDisplayString(withNullability: true),
        ));
    return _generate(model);
  }
}

/// Generator for `@NodeDBView` annotated classes.
class ViewGenerator extends GeneratorForAnnotation<NodeDBView> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@NodeDBView can only be applied to classes.',
        element: element,
      );
    }

    final model = _parseViewModel(element, annotation);
    return _generate(model);
  }
}

/// Generator for `@JsonModel` annotated classes.
///
/// Generates only serialization (`_$XFromMap` / `_$XToMap`) — no schema, DAO,
/// or filter extensions. The model is stored as JSONB when used in a collection.
class JsonModelGenerator extends GeneratorForAnnotation<JsonModel> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@JsonModel can only be applied to classes.',
        element: element,
      );
    }

    final className = element.name;

    // Resolve the name: explicit or default to packages/{package}/{ClassName}
    final explicitName = annotation.peek('name')?.stringValue;
    final packageName = element.library.source.uri.pathSegments.isNotEmpty
        ? element.library.source.uri.pathSegments.first
        : 'unknown';
    final resolvedName = explicitName ?? 'packages/$packageName/$className';

    final fields = <FieldInfo>[];
    for (final field in element.fields) {
      if (field.isStatic || field.isSynthetic) continue;
      fields.add(_parseField(field));
    }

    final model = ModelInfo(
      className: className,
      collectionName: resolvedName,
      type: ModelType.collection,
      generateDao: false,
      fields: fields,
      indexes: const [],
    );

    return generateSerialization(model);
  }
}

/// Generator for `@preferences` annotated classes.
class PreferencesAnnotationGenerator extends GeneratorForAnnotation<Preferences> {
  @override
  String generateForAnnotatedElement(
    Element element,
    ConstantReader annotation,
    BuildStep buildStep,
  ) {
    if (element is! ClassElement) {
      throw InvalidGenerationSourceError(
        '@preferences can only be applied to classes.',
        element: element,
      );
    }

    final model = _parseModel(element, annotation, ModelType.preferences);
    return _generate(model);
  }
}

/// Parse class element into [ModelInfo].
ModelInfo _parseModel(
  ClassElement element,
  ConstantReader annotation,
  ModelType type, {
  EdgeInfo? edgeInfo,
}) {
  final className = element.name;
  final collectionName = pluralize(toSnakeCase(className));
  final schema = annotation.peek('schema')?.stringValue;
  final singleton =
      type == ModelType.collection && (annotation.peek('singleton')?.boolValue ?? false);
  final backup =
      type == ModelType.collection ? (annotation.peek('backup')?.boolValue ?? true) : true;

  // Check for class-level annotations
  bool hasNoDao = false;
  bool trimmable = false;
  bool neverTrim = false;
  String? trimPolicy;
  final triggers = <TriggerInfo>[];

  for (final meta in element.metadata) {
    final metaElement = meta.element;
    if (metaElement == null) continue;

    // Detect annotation name — works for both constructor calls (@NeverTrim())
    // and const instances (@neverTrim) by checking enclosingElement name first,
    // then falling back to the type name of the computed constant value.
    var metaName = metaElement.enclosingElement?.name;
    if (metaName == null) {
      final constVal = meta.computeConstantValue();
      metaName = constVal?.type?.getDisplayString(withNullability: false);
    }

    if (metaName == 'NoDao') {
      hasNoDao = true;
    } else if (metaName == 'Trimmable') {
      trimmable = true;
      final constVal = meta.computeConstantValue();
      if (constVal != null) {
        final reader = ConstantReader(constVal);
        trimPolicy = reader.peek('policy')?.stringValue;
      }
    } else if (metaName == 'NeverTrim') {
      neverTrim = true;
    } else if (metaName == 'Trigger') {
      final constVal = meta.computeConstantValue();
      if (constVal != null) {
        final reader = ConstantReader(constVal);
        triggers.add(TriggerInfo(
          event: reader.read('event').stringValue,
          timing: reader.peek('timing')?.stringValue ?? 'after',
          name: reader.peek('name')?.stringValue,
        ));
      }
    }
  }

  final fields = <FieldInfo>[];
  final indexes = <IndexInfo>[];

  for (final field in element.fields) {
    if (field.isStatic || field.isSynthetic) continue;

    final fieldInfo = _parseField(field);
    fields.add(fieldInfo);
    if (fieldInfo.index != null) {
      indexes.add(fieldInfo.index!);
    }
  }

  return ModelInfo(
    className: className,
    collectionName: collectionName,
    type: type,
    schema: schema,
    singleton: singleton,
    generateDao: !hasNoDao,
    fields: fields,
    indexes: indexes,
    edgeInfo: edgeInfo,
    trimmable: trimmable,
    neverTrim: neverTrim,
    trimPolicy: trimPolicy,
    backup: backup,
    triggers: triggers,
  );
}

/// Parse a @NodeDBView annotated class into [ModelInfo].
ModelInfo _parseViewModel(ClassElement element, ConstantReader annotation) {
  final className = element.name;
  final collectionName = pluralize(toSnakeCase(className));

  // Parse sources
  final sourcesReader = annotation.read('sources');
  final sources = <ViewSourceInfo>[];
  for (final sourceVal in sourcesReader.listValue) {
    final reader = ConstantReader(sourceVal);
    sources.add(ViewSourceInfo(
      collection: reader.read('collection').stringValue,
      schema: reader.peek('schema')?.stringValue ?? 'public',
      database: reader.peek('database')?.stringValue,
    ));
  }

  final strategy = annotation.peek('strategy')?.stringValue ?? 'union';
  final joinField = annotation.peek('joinField')?.stringValue;

  // Check for @noDao
  bool hasNoDao = false;
  for (final meta in element.metadata) {
    final metaElement = meta.element;
    if (metaElement == null) continue;
    var metaName = metaElement.enclosingElement?.name;
    if (metaName == null) {
      final constVal = meta.computeConstantValue();
      metaName = constVal?.type?.getDisplayString(withNullability: false);
    }
    if (metaName == 'NoDao') hasNoDao = true;
  }

  final fields = <FieldInfo>[];
  for (final field in element.fields) {
    if (field.isStatic || field.isSynthetic) continue;
    fields.add(_parseField(field));
  }

  return ModelInfo(
    className: className,
    collectionName: collectionName,
    type: ModelType.view,
    generateDao: !hasNoDao,
    fields: fields,
    indexes: const [],
    viewInfo: ViewInfo(
      sources: sources,
      strategy: strategy,
      joinField: joinField,
    ),
  );
}

/// Parse a single field element.
FieldInfo _parseField(FieldElement field) {
  final name = field.name;
  final dartType = field.type;
  final isNullable = dartType.nullabilitySuffix == NullabilitySuffix.question;

  // Strip nullability for type name
  final typeName = dartType.getDisplayString(withNullability: true).replaceAll('?', '');

  // Check for list types
  bool isList = false;
  String? listElementType;
  if (dartType is InterfaceType && dartType.isDartCoreList) {
    isList = true;
    if (dartType.typeArguments.isNotEmpty) {
      listElementType = dartType.typeArguments.first.getDisplayString(withNullability: true);
    }
  }

  // Check annotations
  IndexInfo? index;
  VectorFieldInfo? vectorField;
  bool isEmbedded = false;
  bool isEnumerated = false;
  bool isJsonb = false;
  String? jsonbSchema;
  String? jsonbIdentifier;

  for (final annotation in field.metadata) {
    final annotationElement = annotation.element;
    if (annotationElement == null) continue;

    var annotationName = annotationElement.enclosingElement?.name;
    if (annotationName == null) {
      final constVal = annotation.computeConstantValue();
      annotationName = constVal?.type?.getDisplayString(withNullability: false);
    }

    if (annotationName == 'Index') {
      final reader = ConstantReader(annotation.computeConstantValue()!);
      index = IndexInfo(
        type: reader.peek('type')?.revive().accessor ?? 'value',
        unique: reader.peek('unique')?.boolValue ?? false,
        composite: reader
                .peek('composite')
                ?.listValue
                .map((e) => e.toStringValue()!)
                .toList() ??
            [],
      );
    } else if (annotationName == 'VectorField') {
      final reader = ConstantReader(annotation.computeConstantValue()!);
      vectorField = VectorFieldInfo(
        dimensions: reader.read('dimensions').intValue,
        metric: reader.peek('metric')?.stringValue ?? 'cosine',
      );
    } else if (annotationName == 'Embedded') {
      isEmbedded = true;
    } else if (annotationName == 'Enumerated') {
      isEnumerated = true;
    } else if (annotationName == 'Jsonb') {
      isJsonb = true;
      final constVal = annotation.computeConstantValue();
      if (constVal != null) {
        final reader = ConstantReader(constVal);
        jsonbSchema = reader.peek('schema')?.stringValue;
        jsonbIdentifier = reader.peek('identifier')?.stringValue;
      }
    }
  }

  // Check if the field's type is a @JsonModel annotated class
  bool isJsonModel = false;
  String? jsonModelType;
  final typeElement = dartType.element;
  if (typeElement is ClassElement) {
    for (final meta in typeElement.metadata) {
      final metaElement = meta.element;
      if (metaElement == null) continue;
      var metaName = metaElement.enclosingElement?.name;
      if (metaName == null) {
        final constVal = meta.computeConstantValue();
        metaName = constVal?.type?.getDisplayString(withNullability: false);
      }
      if (metaName == 'JsonModel') {
        isJsonModel = true;
        jsonModelType = typeElement.name;
        break;
      }
    }
  }

  return FieldInfo(
    name: name,
    dartType: isList ? 'List<${listElementType ?? 'dynamic'}>' : typeName,
    isNullable: isNullable,
    isEmbedded: isEmbedded,
    isEnumerated: isEnumerated,
    isJsonb: isJsonb,
    isJsonModel: isJsonModel,
    jsonModelType: jsonModelType,
    jsonbSchema: jsonbSchema,
    jsonbIdentifier: jsonbIdentifier,
    isList: isList,
    listElementType: listElementType,
    index: index,
    vectorField: vectorField,
  );
}

/// Generate all code for a parsed model.
String _generate(ModelInfo model) {
  final buffer = StringBuffer();

  if (model.type == ModelType.preferences) {
    // Preferences: serialization + preferences accessor
    buffer.writeln(generateSerialization(model));
    buffer.writeln();
    buffer.writeln(generatePreferencesDao(model));
    buffer.writeln();
    buffer.writeln(_generateConcretePrefsDao(model));
    return buffer.toString();
  }

  if (model.type == ModelType.view) {
    // Views: serialization + filter extensions + view DAO (read-only)
    buffer.writeln(generateSchema(model));
    buffer.writeln();
    buffer.writeln(generateSerialization(model));
    buffer.writeln();
    buffer.writeln(generateFilterExtensions(model));
    buffer.writeln();
    if (model.generateDao) {
      buffer.writeln(generateViewDao(model));
      buffer.writeln();
      buffer.writeln(generateConcreteViewDao(model));
    }
    return buffer.toString();
  }

  // Schema
  buffer.writeln(generateSchema(model));
  buffer.writeln();

  // Serialization (fromMap / toMap)
  buffer.writeln(generateSerialization(model));
  buffer.writeln();

  // Filter extensions
  buffer.writeln(generateFilterExtensions(model));
  buffer.writeln();

  // DAO base class + concrete DAO
  if (model.generateDao) {
    buffer.writeln(generateDao(model));
    buffer.writeln();
    buffer.writeln(generateConcreteDao(model));
  }

  return buffer.toString();
}

String _generateConcretePrefsDao(ModelInfo model) {
  final cls = model.className;
  return '''
class ${cls}Prefs extends ${cls}PrefsBase {
  @override
  final NoSqlEngine engine;
  ${cls}Prefs(this.engine);
}
''';
}
