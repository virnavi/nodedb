import 'package:nodedb/nodedb.dart';

import '../models/test_record.dart';
import 'db_adapter.dart';

class NodeDbAdapter extends DbAdapter {
  @override
  String get name => 'NodeDB';

  late NodeDB _db;

  @override
  Future<void> open(String path) async {
    _db = NodeDB.open(
      directory: '$path/nodedb_bench',
      databaseName: 'bench',
    );
  }

  @override
  Future<void> close() async {
    _db.close();
  }

  @override
  Future<void> clear() async {
    _db.nosql.clear('bench.records');
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    _db.nosql.batchPut(
      'bench.records',
      records.map((r) => {'id': r.id, 'data': r.toMap()}).toList(),
    );
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final doc = _db.nosql.get('bench.records', id);
    if (doc == null) return null;
    return TestRecord.fromMap(doc.data);
  }

  @override
  Future<List<TestRecord>> getAll() async {
    final docs = _db.nosql.findAll('bench.records');
    return docs.map((d) => TestRecord.fromMap(d.data)).toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    final docs = _db.nosql.findAll(
      'bench.records',
      filter: {
        'Condition': {
          'GreaterThan': {'field': 'age', 'value': minAge},
        },
      },
    );
    return docs.map((d) => TestRecord.fromMap(d.data)).toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    final docs = _db.nosql.findAll(
      'bench.records',
      filter: {
        'Condition': {
          'Contains': {'field': 'name', 'value': query},
        },
      },
    );
    return docs.map((d) => TestRecord.fromMap(d.data)).toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    _db.nosql.batchPut(
      'bench.records',
      records.map((r) => {'id': r.id, 'data': r.toMap()}).toList(),
    );
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    _db.nosql.batchDelete('bench.records', ids);
  }
}
