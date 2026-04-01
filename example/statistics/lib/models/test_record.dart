import 'dart:math';

class TestRecord {
  final int id;
  final String name;
  final String email;
  final int age;
  final double score;
  final DateTime createdAt;

  TestRecord({
    required this.id,
    required this.name,
    required this.email,
    required this.age,
    required this.score,
    required this.createdAt,
  });

  TestRecord copyWith({
    int? id,
    String? name,
    String? email,
    int? age,
    double? score,
    DateTime? createdAt,
  }) {
    return TestRecord(
      id: id ?? this.id,
      name: name ?? this.name,
      email: email ?? this.email,
      age: age ?? this.age,
      score: score ?? this.score,
      createdAt: createdAt ?? this.createdAt,
    );
  }

  Map<String, dynamic> toMap() => {
        'id': id,
        'name': name,
        'email': email,
        'age': age,
        'score': score,
        'createdAt': createdAt.millisecondsSinceEpoch,
      };

  factory TestRecord.fromMap(Map<String, dynamic> map) => TestRecord(
        id: map['id'] as int,
        name: map['name'] as String,
        email: map['email'] as String,
        age: map['age'] as int,
        score: (map['score'] as num).toDouble(),
        createdAt:
            DateTime.fromMillisecondsSinceEpoch(map['createdAt'] as int),
      );

  static List<TestRecord> generate(int count) {
    final rng = Random(42);
    final now = DateTime.now();
    return List.generate(count, (i) {
      final id = i + 1;
      return TestRecord(
        id: id,
        name: 'User $id',
        email: 'user$id@example.com',
        age: 18 + rng.nextInt(62),
        score: (rng.nextDouble() * 100).roundToDouble() / 10,
        createdAt: now.subtract(Duration(seconds: rng.nextInt(86400 * 365))),
      );
    });
  }
}
