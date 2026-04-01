import 'package:hive_ce/hive_ce.dart';

import '../models/test_record.dart';
import 'db_adapter.dart';

class HiveAdapter extends DbAdapter {
  @override
  String get name => 'Hive CE';

  late Box<Map> _box;

  @override
  Future<void> open(String path) async {
    Hive.init('$path/hive_bench');
    _box = await Hive.openBox<Map>('records');
  }

  @override
  Future<void> close() async {
    await _box.close();
  }

  @override
  Future<void> clear() async {
    await _box.clear();
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    final entries = <int, Map>{};
    for (final r in records) {
      entries[r.id] = r.toMap();
    }
    await _box.putAll(entries);
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final map = _box.get(id);
    if (map == null) return null;
    return TestRecord.fromMap(Map<String, dynamic>.from(map));
  }

  @override
  Future<List<TestRecord>> getAll() async {
    return _box.values
        .map((m) => TestRecord.fromMap(Map<String, dynamic>.from(m)))
        .toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    return _box.values
        .where((m) => (m['age'] as int) > minAge)
        .map((m) => TestRecord.fromMap(Map<String, dynamic>.from(m)))
        .toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    return _box.values
        .where((m) => (m['name'] as String).contains(query))
        .map((m) => TestRecord.fromMap(Map<String, dynamic>.from(m)))
        .toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    final entries = <int, Map>{};
    for (final r in records) {
      entries[r.id] = r.toMap();
    }
    await _box.putAll(entries);
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    await _box.deleteAll(ids);
  }
}
