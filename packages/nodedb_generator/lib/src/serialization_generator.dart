// ignore_for_file: prefer_single_quotes
import 'model_info.dart';

/// Generates `fromMap` and `toMap` methods for a model.
String generateSerialization(ModelInfo model) {
  final buffer = StringBuffer();
  final cls = model.className;

  buffer.writeln('// ── Serialization ──────────────────────────────────────────────');

  // fromMap — static method
  buffer.writeln('$cls _\$${cls}FromMap(Map<String, dynamic> map) {');
  buffer.writeln('  return $cls(');
  for (final field in model.fields) {
    buffer.writeln('    ${field.name}: ${_deserializeField(field)},');
  }
  buffer.writeln('  );');
  buffer.writeln('}');
  buffer.writeln();

  // toMap — extension method
  buffer.writeln('Map<String, dynamic> _\$${cls}ToMap($cls instance) {');
  buffer.writeln('  return {');
  for (final field in model.fields) {
    buffer.writeln("    '${field.name}': ${_serializeField(field)},");
  }
  buffer.writeln('  };');
  buffer.writeln('}');

  return buffer.toString();
}

String _deserializeField(FieldInfo field) {
  final key = "map['${field.name}']";
  final type = field.dartType;

  // JsonModel fields need special handling before the generic nullable check
  if (field.isJsonModel) {
    final t = field.jsonModelType!;
    if (field.isNullable) {
      return "$key != null ? _\$${t}FromMap(Map<String, dynamic>.from($key as Map)) : null";
    }
    return "_\$${t}FromMap(Map<String, dynamic>.from($key as Map? ?? {}))";
  }

  if (field.isNullable) {
    if (field.isDateTime) {
      return "$key != null ? DateTime.parse($key as String) : null";
    }
    if (field.isEnumerated) {
      return "$key != null ? ${type.replaceAll('?', '')}.values.firstWhere("
          "(e) => e.name == $key, orElse: () => ${type.replaceAll('?', '')}.values.first) : null";
    }
    if (field.isJsonb || field.isMap) {
      return "$key != null ? Map<String, dynamic>.from($key as Map) : null";
    }
    if (field.isList) {
      final elementType = field.listElementType ?? 'dynamic';
      if (elementType == 'double') {
        return "$key != null ? ($key as List).cast<num>().map((e) => e.toDouble()).toList() : null";
      }
      return "$key != null ? ($key as List).cast<$elementType>() : null";
    }
    return '$key as $type?';
  }

  if (field.isDateTime) {
    return "DateTime.parse($key as String)";
  }

  if (field.isEnumerated) {
    return "$type.values.firstWhere((e) => e.name == $key, "
        "orElse: () => $type.values.first)";
  }

  if (field.isJsonb || field.isMap) {
    return "Map<String, dynamic>.from($key as Map? ?? {})";
  }

  if (field.isList) {
    final elementType = field.listElementType ?? 'dynamic';
    if (elementType == 'double') {
      return "($key as List? ?? []).cast<num>().map((e) => e.toDouble()).toList()";
    }
    return "($key as List? ?? []).cast<$elementType>()";
  }

  // Numeric types need careful casting (msgpack may return int for double)
  if (type == 'double') {
    return "($key as num).toDouble()";
  }
  if (type == 'int') {
    return "($key as num).toInt()";
  }

  return '$key as $type';
}

String _serializeField(FieldInfo field) {
  final accessor = 'instance.${field.name}';

  if (field.isDateTime) {
    if (field.isNullable) {
      return '$accessor?.toIso8601String()';
    }
    return '$accessor.toIso8601String()';
  }

  if (field.isEnumerated) {
    if (field.isNullable) {
      return '$accessor?.name';
    }
    return '$accessor.name';
  }

  if (field.isJsonModel) {
    final t = field.jsonModelType!;
    if (field.isNullable) {
      return '$accessor != null ? _\$${t}ToMap($accessor!) : null';
    }
    return '_\$${t}ToMap($accessor)';
  }

  return accessor;
}
