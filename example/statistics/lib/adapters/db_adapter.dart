import '../models/test_record.dart';

abstract class DbAdapter {
  String get name;

  Future<void> open(String path);
  Future<void> close();
  Future<void> clear();

  Future<void> insertBatch(List<TestRecord> records);
  Future<TestRecord?> getById(int id);
  Future<List<TestRecord>> getAll();
  Future<List<TestRecord>> queryByAge(int minAge);
  Future<List<TestRecord>> searchByName(String query);
  Future<void> updateBatch(List<TestRecord> records);
  Future<void> deleteBatch(List<int> ids);
}
