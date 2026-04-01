import 'model_info.dart';

/// Generates a typed preferences accessor class.
///
/// For each field in the model, generates get/set/remove methods
/// that delegate to [NoSqlEngine.prefGet/prefSet/prefRemove].
String generatePreferencesDao(ModelInfo model) {
  final cls = model.className;
  final daoName = '${cls}PrefsBase';
  final storeName = model.collectionName;

  final buffer = StringBuffer();

  buffer.writeln(
      '// ── Preferences Accessor ───────────────────────────────────────');
  buffer.writeln('abstract class $daoName {');
  buffer.writeln('  NoSqlEngine get engine;');
  buffer.writeln();
  buffer.writeln("  String get storeName => '$storeName';");
  buffer.writeln();

  for (final field in model.dataFields) {
    final name = field.name;
    final type = field.dartType;

    // Getter — always nullable since the preference may not be set
    buffer.writeln('  $type? get${_capitalize(name)}() {');
    buffer.writeln("    final resp = engine.prefGet(storeName, '$name');");
    buffer.writeln(
        "    if (resp is Map && resp['found'] == true) return resp['value'] as $type;");
    buffer.writeln('    return null;');
    buffer.writeln('  }');
    buffer.writeln();

    // Setter
    buffer.writeln(
        '  void set${_capitalize(name)}($type value, {bool shareable = false}) {');
    buffer.writeln(
        "    engine.prefSet(storeName, '$name', value, shareable: shareable);");
    buffer.writeln('  }');
    buffer.writeln();

    // Remover
    buffer.writeln('  bool remove${_capitalize(name)}() {');
    buffer.writeln("    return engine.prefRemove(storeName, '$name');");
    buffer.writeln('  }');
    buffer.writeln();
  }

  // Utility methods
  buffer.writeln('  List<String> allKeys() => engine.prefKeys(storeName);');
  buffer.writeln();
  buffer.writeln(
      '  List<Map<String, dynamic>> shareableEntries() => engine.prefShareable(storeName);');
  buffer.writeln();

  // removeAll
  buffer.writeln('  void removeAll() {');
  buffer.writeln('    for (final key in allKeys()) {');
  buffer.writeln('      engine.prefRemove(storeName, key);');
  buffer.writeln('    }');
  buffer.writeln('  }');

  buffer.writeln('}');

  return buffer.toString();
}

String _capitalize(String s) {
  if (s.isEmpty) return s;
  return s[0].toUpperCase() + s.substring(1);
}
