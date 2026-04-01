import 'package:sqflite/sqflite.dart';

import '../models/test_record.dart';
import 'db_adapter.dart';

class SqfliteAdapter extends DbAdapter {
  @override
  String get name => 'sqflite';

  late Database _db;

  @override
  Future<void> open(String path) async {
    _db = await openDatabase(
      '$path/sqflite_bench.db',
      version: 1,
      onCreate: (db, version) async {
        await db.execute('''
          CREATE TABLE records (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            age INTEGER NOT NULL,
            score REAL NOT NULL,
            createdAt INTEGER NOT NULL
          )
        ''');
        await db.execute(
            'CREATE INDEX idx_records_age ON records (age)');
      },
    );
  }

  @override
  Future<void> close() async {
    await _db.close();
  }

  @override
  Future<void> clear() async {
    await _db.delete('records');
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    final batch = _db.batch();
    for (final r in records) {
      batch.insert('records', r.toMap(), conflictAlgorithm: ConflictAlgorithm.replace);
    }
    await batch.commit(noResult: true);
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final rows = await _db.query('records', where: 'id = ?', whereArgs: [id]);
    if (rows.isEmpty) return null;
    return TestRecord.fromMap(rows.first);
  }

  @override
  Future<List<TestRecord>> getAll() async {
    final rows = await _db.query('records');
    return rows.map(TestRecord.fromMap).toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    final rows =
        await _db.query('records', where: 'age > ?', whereArgs: [minAge]);
    return rows.map(TestRecord.fromMap).toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    final rows = await _db.query('records',
        where: 'name LIKE ?', whereArgs: ['%$query%']);
    return rows.map(TestRecord.fromMap).toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    final batch = _db.batch();
    for (final r in records) {
      batch.update('records', r.toMap(), where: 'id = ?', whereArgs: [r.id]);
    }
    await batch.commit(noResult: true);
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    final batch = _db.batch();
    for (final id in ids) {
      batch.delete('records', where: 'id = ?', whereArgs: [id]);
    }
    await batch.commit(noResult: true);
  }
}
