import 'package:nodedb/nodedb.dart';

/// Seed data helpers for test setup.
class TestSeed {
  final NoSqlEngine _engine;

  TestSeed(this._engine);

  /// Insert a batch of documents into a collection.
  List<Document> insertAll(String collection, List<Map<String, dynamic>> docs) {
    _engine.writeTxn(
      docs.map((d) => WriteOp.put(collection, data: d)).toList(),
    );
    return _engine.findAll(collection);
  }

  /// Insert a single document and return it.
  Document insert(String collection, Map<String, dynamic> data) {
    _engine.writeTxn([WriteOp.put(collection, data: data)]);
    final all = _engine.findAll(collection);
    return all.last;
  }

  /// Generate N documents with an incrementing field.
  ///
  /// ```dart
  /// seed.generate('users', 10, (i) => {'name': 'User $i', 'age': 20 + i});
  /// ```
  List<Document> generate(
    String collection,
    int count,
    Map<String, dynamic> Function(int index) builder,
  ) {
    final docs = List.generate(count, builder);
    return insertAll(collection, docs);
  }

  /// Seed common test collections: users, posts, tags.
  void seedSampleData() {
    insertAll('users', [
      {'name': 'Alice', 'age': 30, 'role': 'admin'},
      {'name': 'Bob', 'age': 25, 'role': 'user'},
      {'name': 'Charlie', 'age': 35, 'role': 'user'},
      {'name': 'Diana', 'age': 28, 'role': 'moderator'},
    ]);

    insertAll('posts', [
      {'title': 'Hello World', 'author': 'Alice', 'draft': false},
      {'title': 'Dart Tips', 'author': 'Bob', 'draft': true},
      {'title': 'NodeDB Guide', 'author': 'Alice', 'draft': false},
    ]);
  }
}
