import 'package:isar_community/isar.dart';

import '../models/test_record.dart';
import 'db_adapter.dart';

part 'isar_adapter.g.dart';

@collection
class IsarRecord {
  Id id = Isar.autoIncrement;

  @Index()
  int age = 0;

  late String name;
  late String email;
  double score = 0.0;
  DateTime createdAt = DateTime.now();
}

class IsarAdapter extends DbAdapter {
  @override
  String get name => 'Isar';

  late Isar _isar;

  @override
  Future<void> open(String path) async {
    _isar = await Isar.open(
      [IsarRecordSchema],
      directory: '$path/isar_bench',
    );
  }

  @override
  Future<void> close() async {
    await _isar.close();
  }

  @override
  Future<void> clear() async {
    await _isar.writeTxn(() => _isar.isarRecords.clear());
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    final entities = records.map(_toEntity).toList();
    await _isar.writeTxn(() => _isar.isarRecords.putAll(entities));
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final entity = await _isar.isarRecords.get(id);
    if (entity == null) return null;
    return _fromEntity(entity);
  }

  @override
  Future<List<TestRecord>> getAll() async {
    final entities = await _isar.isarRecords.where().findAll();
    return entities.map(_fromEntity).toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    final entities = await _isar.isarRecords
        .filter()
        .ageGreaterThan(minAge)
        .findAll();
    return entities.map(_fromEntity).toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    final entities = await _isar.isarRecords
        .filter()
        .nameContains(query)
        .findAll();
    return entities.map(_fromEntity).toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    final entities = records.map(_toEntity).toList();
    await _isar.writeTxn(() => _isar.isarRecords.putAll(entities));
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    await _isar.writeTxn(() => _isar.isarRecords.deleteAll(ids));
  }

  IsarRecord _toEntity(TestRecord r) => IsarRecord()
    ..id = r.id
    ..name = r.name
    ..email = r.email
    ..age = r.age
    ..score = r.score
    ..createdAt = r.createdAt;

  TestRecord _fromEntity(IsarRecord e) => TestRecord(
        id: e.id,
        name: e.name,
        email: e.email,
        age: e.age,
        score: e.score,
        createdAt: e.createdAt,
      );
}
