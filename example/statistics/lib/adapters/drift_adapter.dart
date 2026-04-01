import 'dart:io';

import 'package:drift/drift.dart';
import 'package:drift/native.dart';

import '../models/test_record.dart';
import 'db_adapter.dart';

part 'drift_adapter.g.dart';

class DriftRecords extends Table {
  IntColumn get id => integer()();
  TextColumn get name => text()();
  TextColumn get email => text()();
  IntColumn get age => integer()();
  RealColumn get score => real()();
  IntColumn get createdAt => integer()();

  @override
  Set<Column> get primaryKey => {id};

  @override
  String get tableName => 'records';
}

@DriftDatabase(tables: [DriftRecords])
class BenchmarkDatabase extends _$BenchmarkDatabase {
  BenchmarkDatabase(String path)
      : super(NativeDatabase(File('$path/drift_bench.db')));

  @override
  int get schemaVersion => 1;

  @override
  MigrationStrategy get migration => MigrationStrategy(
        onCreate: (m) async {
          await m.createAll();
          await customStatement(
              'CREATE INDEX IF NOT EXISTS idx_records_age ON records (age)');
        },
      );
}

class DriftAdapter extends DbAdapter {
  @override
  String get name => 'Drift';

  late BenchmarkDatabase _db;

  @override
  Future<void> open(String path) async {
    _db = BenchmarkDatabase(path);
    // Force table creation
    await _db.customSelect('SELECT 1').get();
  }

  @override
  Future<void> close() async {
    await _db.close();
  }

  @override
  Future<void> clear() async {
    await _db.delete(_db.driftRecords).go();
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    await _db.batch((batch) {
      batch.insertAll(
        _db.driftRecords,
        records.map((r) => DriftRecordsCompanion.insert(
              id: Value(r.id),
              name: r.name,
              email: r.email,
              age: r.age,
              score: r.score,
              createdAt: r.createdAt.millisecondsSinceEpoch,
            )),
        mode: InsertMode.insertOrReplace,
      );
    });
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final row = await (_db.select(_db.driftRecords)
          ..where((r) => r.id.equals(id)))
        .getSingleOrNull();
    if (row == null) return null;
    return _fromRow(row);
  }

  @override
  Future<List<TestRecord>> getAll() async {
    final rows = await _db.select(_db.driftRecords).get();
    return rows.map(_fromRow).toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    final rows = await (_db.select(_db.driftRecords)
          ..where((r) => r.age.isBiggerThanValue(minAge)))
        .get();
    return rows.map(_fromRow).toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    final rows = await (_db.select(_db.driftRecords)
          ..where((r) => r.name.contains(query)))
        .get();
    return rows.map(_fromRow).toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    await _db.batch((batch) {
      for (final r in records) {
        batch.replace(
          _db.driftRecords,
          DriftRecordsCompanion(
            id: Value(r.id),
            name: Value(r.name),
            email: Value(r.email),
            age: Value(r.age),
            score: Value(r.score),
            createdAt: Value(r.createdAt.millisecondsSinceEpoch),
          ),
        );
      }
    });
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    await (_db.delete(_db.driftRecords)..where((r) => r.id.isIn(ids))).go();
  }

  TestRecord _fromRow(DriftRecord row) => TestRecord(
        id: row.id,
        name: row.name,
        email: row.email,
        age: row.age,
        score: row.score,
        createdAt: DateTime.fromMillisecondsSinceEpoch(row.createdAt),
      );
}
