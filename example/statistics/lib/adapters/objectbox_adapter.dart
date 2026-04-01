import '../models/test_record.dart';
import 'db_adapter.dart';
import '../objectbox.g.dart';

@Entity()
class RecordEntity {
  @Id()
  int obxId = 0;

  @Unique()
  int recordId;
  String name;
  String email;
  int age;
  double score;

  @Property(type: PropertyType.dateNano)
  DateTime createdAt;

  RecordEntity({
    required this.recordId,
    required this.name,
    required this.email,
    required this.age,
    required this.score,
    required this.createdAt,
  });
}

class ObjectBoxAdapter extends DbAdapter {
  @override
  String get name => 'ObjectBox';

  late Store _store;
  late Box<RecordEntity> _box;

  @override
  Future<void> open(String path) async {
    _store = await openStore(directory: '$path/objectbox_bench');
    _box = _store.box<RecordEntity>();
  }

  @override
  Future<void> close() async {
    _store.close();
  }

  @override
  Future<void> clear() async {
    _box.removeAll();
  }

  @override
  Future<void> insertBatch(List<TestRecord> records) async {
    final entities = records
        .map((r) => RecordEntity(
              recordId: r.id,
              name: r.name,
              email: r.email,
              age: r.age,
              score: r.score,
              createdAt: r.createdAt,
            ))
        .toList();
    _box.putMany(entities);
  }

  @override
  Future<TestRecord?> getById(int id) async {
    final query =
        _box.query(RecordEntity_.recordId.equals(id)).build();
    final entity = query.findFirst();
    query.close();
    if (entity == null) return null;
    return _fromEntity(entity);
  }

  @override
  Future<List<TestRecord>> getAll() async {
    return _box.getAll().map(_fromEntity).toList();
  }

  @override
  Future<List<TestRecord>> queryByAge(int minAge) async {
    final query =
        _box.query(RecordEntity_.age.greaterThan(minAge)).build();
    final results = query.find();
    query.close();
    return results.map(_fromEntity).toList();
  }

  @override
  Future<List<TestRecord>> searchByName(String query) async {
    final q = _box.query(RecordEntity_.name.contains(query)).build();
    final results = q.find();
    q.close();
    return results.map(_fromEntity).toList();
  }

  @override
  Future<void> updateBatch(List<TestRecord> records) async {
    final entities = <RecordEntity>[];
    for (final r in records) {
      final query =
          _box.query(RecordEntity_.recordId.equals(r.id)).build();
      final existing = query.findFirst();
      query.close();
      final entity = RecordEntity(
        recordId: r.id,
        name: r.name,
        email: r.email,
        age: r.age,
        score: r.score,
        createdAt: r.createdAt,
      );
      if (existing != null) {
        entity.obxId = existing.obxId;
      }
      entities.add(entity);
    }
    _box.putMany(entities);
  }

  @override
  Future<void> deleteBatch(List<int> ids) async {
    for (final id in ids) {
      final query =
          _box.query(RecordEntity_.recordId.equals(id)).build();
      final entity = query.findFirst();
      query.close();
      if (entity != null) {
        _box.remove(entity.obxId);
      }
    }
  }

  TestRecord _fromEntity(RecordEntity e) => TestRecord(
        id: e.recordId,
        name: e.name,
        email: e.email,
        age: e.age,
        score: e.score,
        createdAt: e.createdAt,
      );
}
