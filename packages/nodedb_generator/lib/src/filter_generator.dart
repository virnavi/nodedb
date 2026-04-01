import 'model_info.dart';

/// Generates typed FilterQuery extension methods for a model.
///
/// For each field, generates type-appropriate filter methods that
/// delegate to the generic FilterQuery condition builders.
String generateFilterExtensions(ModelInfo model) {
  final buffer = StringBuffer();
  final cls = model.className;

  buffer.writeln('// ── Filter Extensions ───────────────────────────────────────────');
  buffer.writeln('extension ${cls}FilterExtension on FilterQuery<$cls> {');

  for (final field in model.fields) {
    _generateFieldFilters(buffer, field, cls);
  }

  // Sort helpers
  for (final field in model.fields) {
    if (field.isEmbedded || field.isList || field.isJsonbLike || field.isMap) continue;
    final capitalized = _capitalize(field.name);
    buffer.writeln(
        '  FilterQuery<$cls> sortBy$capitalized({bool desc = false}) => '
        "sortBy('${field.name}', desc: desc);");
  }

  buffer.writeln('}');
  return buffer.toString();
}

void _generateFieldFilters(StringBuffer buffer, FieldInfo field, String cls) {
  final name = field.name;

  // JSONB / Map fields get path-based operators
  if (field.isJsonbLike || field.isMap) {
    buffer.writeln(
        '  FilterQuery<$cls> ${name}PathEquals(String path, dynamic value) => '
        "jsonPathEquals('$name', path, value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}HasKey(String path) => '
        "jsonHasKey('$name', path);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}Contains(Map<String, dynamic> value) => '
        "jsonContains('$name', value);");
    return;
  }

  // List fields get array operators
  if (field.isList) {
    final elementType = field.listElementType ?? 'dynamic';
    buffer.writeln(
        '  FilterQuery<$cls> ${name}Contains($elementType value) => '
        "arrayContains('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}Overlaps(List<$elementType> values) => '
        "arrayOverlap('$name', values);");
    return;
  }

  // Skip embedded fields — they don't have simple filter predicates
  if (field.isEmbedded) return;

  // All types get equalTo / notEqualTo / inList / notInList
  buffer.writeln(
      '  FilterQuery<$cls> ${name}EqualTo(${field.dartType} value) => '
      "equalTo('$name', value);");
  buffer.writeln(
      '  FilterQuery<$cls> ${name}NotEqualTo(${field.dartType} value) => '
      "notEqualTo('$name', value);");
  buffer.writeln(
      '  FilterQuery<$cls> ${name}InList(List<${field.dartType}> values) => '
      "inList('$name', values);");
  buffer.writeln(
      '  FilterQuery<$cls> ${name}NotInList(List<${field.dartType}> values) => '
      "notInList('$name', values);");

  // Nullable fields get isNull / isNotNull
  if (field.isNullable) {
    buffer.writeln(
        '  FilterQuery<$cls> ${name}IsNull() => '
        "isNull('$name');");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}IsNotNull() => '
        "isNotNull('$name');");
  }

  // Numeric and DateTime fields get comparison operators
  if (field.isNumeric || field.isDateTime) {
    final argType = field.dartType;
    buffer.writeln(
        '  FilterQuery<$cls> ${name}GreaterThan($argType value) => '
        "greaterThan('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}GreaterThanOrEqual($argType value) => '
        "greaterThanOrEqual('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}LessThan($argType value) => '
        "lessThan('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}LessThanOrEqual($argType value) => '
        "lessThanOrEqual('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}Between($argType low, $argType high) => '
        "between('$name', low, high);");
  }

  // String fields get text-specific operators
  if (field.isString) {
    buffer.writeln(
        '  FilterQuery<$cls> ${name}Contains(String value) => '
        "contains('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}StartsWith(String value) => '
        "startsWith('$name', value);");
    buffer.writeln(
        '  FilterQuery<$cls> ${name}EndsWith(String value) => '
        "endsWith('$name', value);");
  }
}

String _capitalize(String s) => s.isEmpty ? s : '${s[0].toUpperCase()}${s.substring(1)}';
