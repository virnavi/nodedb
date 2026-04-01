// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'drift_adapter.dart';

// ignore_for_file: type=lint
class $DriftRecordsTable extends DriftRecords
    with TableInfo<$DriftRecordsTable, DriftRecord> {
  @override
  final GeneratedDatabase attachedDatabase;
  final String? _alias;
  $DriftRecordsTable(this.attachedDatabase, [this._alias]);
  static const VerificationMeta _idMeta = const VerificationMeta('id');
  @override
  late final GeneratedColumn<int> id = GeneratedColumn<int>(
    'id',
    aliasedName,
    false,
    type: DriftSqlType.int,
    requiredDuringInsert: false,
  );
  static const VerificationMeta _nameMeta = const VerificationMeta('name');
  @override
  late final GeneratedColumn<String> name = GeneratedColumn<String>(
    'name',
    aliasedName,
    false,
    type: DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const VerificationMeta _emailMeta = const VerificationMeta('email');
  @override
  late final GeneratedColumn<String> email = GeneratedColumn<String>(
    'email',
    aliasedName,
    false,
    type: DriftSqlType.string,
    requiredDuringInsert: true,
  );
  static const VerificationMeta _ageMeta = const VerificationMeta('age');
  @override
  late final GeneratedColumn<int> age = GeneratedColumn<int>(
    'age',
    aliasedName,
    false,
    type: DriftSqlType.int,
    requiredDuringInsert: true,
  );
  static const VerificationMeta _scoreMeta = const VerificationMeta('score');
  @override
  late final GeneratedColumn<double> score = GeneratedColumn<double>(
    'score',
    aliasedName,
    false,
    type: DriftSqlType.double,
    requiredDuringInsert: true,
  );
  static const VerificationMeta _createdAtMeta = const VerificationMeta(
    'createdAt',
  );
  @override
  late final GeneratedColumn<int> createdAt = GeneratedColumn<int>(
    'created_at',
    aliasedName,
    false,
    type: DriftSqlType.int,
    requiredDuringInsert: true,
  );
  @override
  List<GeneratedColumn> get $columns => [
    id,
    name,
    email,
    age,
    score,
    createdAt,
  ];
  @override
  String get aliasedName => _alias ?? actualTableName;
  @override
  String get actualTableName => $name;
  static const String $name = 'records';
  @override
  VerificationContext validateIntegrity(
    Insertable<DriftRecord> instance, {
    bool isInserting = false,
  }) {
    final context = VerificationContext();
    final data = instance.toColumns(true);
    if (data.containsKey('id')) {
      context.handle(_idMeta, id.isAcceptableOrUnknown(data['id']!, _idMeta));
    }
    if (data.containsKey('name')) {
      context.handle(
        _nameMeta,
        name.isAcceptableOrUnknown(data['name']!, _nameMeta),
      );
    } else if (isInserting) {
      context.missing(_nameMeta);
    }
    if (data.containsKey('email')) {
      context.handle(
        _emailMeta,
        email.isAcceptableOrUnknown(data['email']!, _emailMeta),
      );
    } else if (isInserting) {
      context.missing(_emailMeta);
    }
    if (data.containsKey('age')) {
      context.handle(
        _ageMeta,
        age.isAcceptableOrUnknown(data['age']!, _ageMeta),
      );
    } else if (isInserting) {
      context.missing(_ageMeta);
    }
    if (data.containsKey('score')) {
      context.handle(
        _scoreMeta,
        score.isAcceptableOrUnknown(data['score']!, _scoreMeta),
      );
    } else if (isInserting) {
      context.missing(_scoreMeta);
    }
    if (data.containsKey('created_at')) {
      context.handle(
        _createdAtMeta,
        createdAt.isAcceptableOrUnknown(data['created_at']!, _createdAtMeta),
      );
    } else if (isInserting) {
      context.missing(_createdAtMeta);
    }
    return context;
  }

  @override
  Set<GeneratedColumn> get $primaryKey => {id};
  @override
  DriftRecord map(Map<String, dynamic> data, {String? tablePrefix}) {
    final effectivePrefix = tablePrefix != null ? '$tablePrefix.' : '';
    return DriftRecord(
      id: attachedDatabase.typeMapping.read(
        DriftSqlType.int,
        data['${effectivePrefix}id'],
      )!,
      name: attachedDatabase.typeMapping.read(
        DriftSqlType.string,
        data['${effectivePrefix}name'],
      )!,
      email: attachedDatabase.typeMapping.read(
        DriftSqlType.string,
        data['${effectivePrefix}email'],
      )!,
      age: attachedDatabase.typeMapping.read(
        DriftSqlType.int,
        data['${effectivePrefix}age'],
      )!,
      score: attachedDatabase.typeMapping.read(
        DriftSqlType.double,
        data['${effectivePrefix}score'],
      )!,
      createdAt: attachedDatabase.typeMapping.read(
        DriftSqlType.int,
        data['${effectivePrefix}created_at'],
      )!,
    );
  }

  @override
  $DriftRecordsTable createAlias(String alias) {
    return $DriftRecordsTable(attachedDatabase, alias);
  }
}

class DriftRecord extends DataClass implements Insertable<DriftRecord> {
  final int id;
  final String name;
  final String email;
  final int age;
  final double score;
  final int createdAt;
  const DriftRecord({
    required this.id,
    required this.name,
    required this.email,
    required this.age,
    required this.score,
    required this.createdAt,
  });
  @override
  Map<String, Expression> toColumns(bool nullToAbsent) {
    final map = <String, Expression>{};
    map['id'] = Variable<int>(id);
    map['name'] = Variable<String>(name);
    map['email'] = Variable<String>(email);
    map['age'] = Variable<int>(age);
    map['score'] = Variable<double>(score);
    map['created_at'] = Variable<int>(createdAt);
    return map;
  }

  DriftRecordsCompanion toCompanion(bool nullToAbsent) {
    return DriftRecordsCompanion(
      id: Value(id),
      name: Value(name),
      email: Value(email),
      age: Value(age),
      score: Value(score),
      createdAt: Value(createdAt),
    );
  }

  factory DriftRecord.fromJson(
    Map<String, dynamic> json, {
    ValueSerializer? serializer,
  }) {
    serializer ??= driftRuntimeOptions.defaultSerializer;
    return DriftRecord(
      id: serializer.fromJson<int>(json['id']),
      name: serializer.fromJson<String>(json['name']),
      email: serializer.fromJson<String>(json['email']),
      age: serializer.fromJson<int>(json['age']),
      score: serializer.fromJson<double>(json['score']),
      createdAt: serializer.fromJson<int>(json['createdAt']),
    );
  }
  @override
  Map<String, dynamic> toJson({ValueSerializer? serializer}) {
    serializer ??= driftRuntimeOptions.defaultSerializer;
    return <String, dynamic>{
      'id': serializer.toJson<int>(id),
      'name': serializer.toJson<String>(name),
      'email': serializer.toJson<String>(email),
      'age': serializer.toJson<int>(age),
      'score': serializer.toJson<double>(score),
      'createdAt': serializer.toJson<int>(createdAt),
    };
  }

  DriftRecord copyWith({
    int? id,
    String? name,
    String? email,
    int? age,
    double? score,
    int? createdAt,
  }) => DriftRecord(
    id: id ?? this.id,
    name: name ?? this.name,
    email: email ?? this.email,
    age: age ?? this.age,
    score: score ?? this.score,
    createdAt: createdAt ?? this.createdAt,
  );
  DriftRecord copyWithCompanion(DriftRecordsCompanion data) {
    return DriftRecord(
      id: data.id.present ? data.id.value : this.id,
      name: data.name.present ? data.name.value : this.name,
      email: data.email.present ? data.email.value : this.email,
      age: data.age.present ? data.age.value : this.age,
      score: data.score.present ? data.score.value : this.score,
      createdAt: data.createdAt.present ? data.createdAt.value : this.createdAt,
    );
  }

  @override
  String toString() {
    return (StringBuffer('DriftRecord(')
          ..write('id: $id, ')
          ..write('name: $name, ')
          ..write('email: $email, ')
          ..write('age: $age, ')
          ..write('score: $score, ')
          ..write('createdAt: $createdAt')
          ..write(')'))
        .toString();
  }

  @override
  int get hashCode => Object.hash(id, name, email, age, score, createdAt);
  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is DriftRecord &&
          other.id == this.id &&
          other.name == this.name &&
          other.email == this.email &&
          other.age == this.age &&
          other.score == this.score &&
          other.createdAt == this.createdAt);
}

class DriftRecordsCompanion extends UpdateCompanion<DriftRecord> {
  final Value<int> id;
  final Value<String> name;
  final Value<String> email;
  final Value<int> age;
  final Value<double> score;
  final Value<int> createdAt;
  const DriftRecordsCompanion({
    this.id = const Value.absent(),
    this.name = const Value.absent(),
    this.email = const Value.absent(),
    this.age = const Value.absent(),
    this.score = const Value.absent(),
    this.createdAt = const Value.absent(),
  });
  DriftRecordsCompanion.insert({
    this.id = const Value.absent(),
    required String name,
    required String email,
    required int age,
    required double score,
    required int createdAt,
  }) : name = Value(name),
       email = Value(email),
       age = Value(age),
       score = Value(score),
       createdAt = Value(createdAt);
  static Insertable<DriftRecord> custom({
    Expression<int>? id,
    Expression<String>? name,
    Expression<String>? email,
    Expression<int>? age,
    Expression<double>? score,
    Expression<int>? createdAt,
  }) {
    return RawValuesInsertable({
      if (id != null) 'id': id,
      if (name != null) 'name': name,
      if (email != null) 'email': email,
      if (age != null) 'age': age,
      if (score != null) 'score': score,
      if (createdAt != null) 'created_at': createdAt,
    });
  }

  DriftRecordsCompanion copyWith({
    Value<int>? id,
    Value<String>? name,
    Value<String>? email,
    Value<int>? age,
    Value<double>? score,
    Value<int>? createdAt,
  }) {
    return DriftRecordsCompanion(
      id: id ?? this.id,
      name: name ?? this.name,
      email: email ?? this.email,
      age: age ?? this.age,
      score: score ?? this.score,
      createdAt: createdAt ?? this.createdAt,
    );
  }

  @override
  Map<String, Expression> toColumns(bool nullToAbsent) {
    final map = <String, Expression>{};
    if (id.present) {
      map['id'] = Variable<int>(id.value);
    }
    if (name.present) {
      map['name'] = Variable<String>(name.value);
    }
    if (email.present) {
      map['email'] = Variable<String>(email.value);
    }
    if (age.present) {
      map['age'] = Variable<int>(age.value);
    }
    if (score.present) {
      map['score'] = Variable<double>(score.value);
    }
    if (createdAt.present) {
      map['created_at'] = Variable<int>(createdAt.value);
    }
    return map;
  }

  @override
  String toString() {
    return (StringBuffer('DriftRecordsCompanion(')
          ..write('id: $id, ')
          ..write('name: $name, ')
          ..write('email: $email, ')
          ..write('age: $age, ')
          ..write('score: $score, ')
          ..write('createdAt: $createdAt')
          ..write(')'))
        .toString();
  }
}

abstract class _$BenchmarkDatabase extends GeneratedDatabase {
  _$BenchmarkDatabase(QueryExecutor e) : super(e);
  $BenchmarkDatabaseManager get managers => $BenchmarkDatabaseManager(this);
  late final $DriftRecordsTable driftRecords = $DriftRecordsTable(this);
  @override
  Iterable<TableInfo<Table, Object?>> get allTables =>
      allSchemaEntities.whereType<TableInfo<Table, Object?>>();
  @override
  List<DatabaseSchemaEntity> get allSchemaEntities => [driftRecords];
}

typedef $$DriftRecordsTableCreateCompanionBuilder =
    DriftRecordsCompanion Function({
      Value<int> id,
      required String name,
      required String email,
      required int age,
      required double score,
      required int createdAt,
    });
typedef $$DriftRecordsTableUpdateCompanionBuilder =
    DriftRecordsCompanion Function({
      Value<int> id,
      Value<String> name,
      Value<String> email,
      Value<int> age,
      Value<double> score,
      Value<int> createdAt,
    });

class $$DriftRecordsTableFilterComposer
    extends Composer<_$BenchmarkDatabase, $DriftRecordsTable> {
  $$DriftRecordsTableFilterComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  ColumnFilters<int> get id => $composableBuilder(
    column: $table.id,
    builder: (column) => ColumnFilters(column),
  );

  ColumnFilters<String> get name => $composableBuilder(
    column: $table.name,
    builder: (column) => ColumnFilters(column),
  );

  ColumnFilters<String> get email => $composableBuilder(
    column: $table.email,
    builder: (column) => ColumnFilters(column),
  );

  ColumnFilters<int> get age => $composableBuilder(
    column: $table.age,
    builder: (column) => ColumnFilters(column),
  );

  ColumnFilters<double> get score => $composableBuilder(
    column: $table.score,
    builder: (column) => ColumnFilters(column),
  );

  ColumnFilters<int> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => ColumnFilters(column),
  );
}

class $$DriftRecordsTableOrderingComposer
    extends Composer<_$BenchmarkDatabase, $DriftRecordsTable> {
  $$DriftRecordsTableOrderingComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  ColumnOrderings<int> get id => $composableBuilder(
    column: $table.id,
    builder: (column) => ColumnOrderings(column),
  );

  ColumnOrderings<String> get name => $composableBuilder(
    column: $table.name,
    builder: (column) => ColumnOrderings(column),
  );

  ColumnOrderings<String> get email => $composableBuilder(
    column: $table.email,
    builder: (column) => ColumnOrderings(column),
  );

  ColumnOrderings<int> get age => $composableBuilder(
    column: $table.age,
    builder: (column) => ColumnOrderings(column),
  );

  ColumnOrderings<double> get score => $composableBuilder(
    column: $table.score,
    builder: (column) => ColumnOrderings(column),
  );

  ColumnOrderings<int> get createdAt => $composableBuilder(
    column: $table.createdAt,
    builder: (column) => ColumnOrderings(column),
  );
}

class $$DriftRecordsTableAnnotationComposer
    extends Composer<_$BenchmarkDatabase, $DriftRecordsTable> {
  $$DriftRecordsTableAnnotationComposer({
    required super.$db,
    required super.$table,
    super.joinBuilder,
    super.$addJoinBuilderToRootComposer,
    super.$removeJoinBuilderFromRootComposer,
  });
  GeneratedColumn<int> get id =>
      $composableBuilder(column: $table.id, builder: (column) => column);

  GeneratedColumn<String> get name =>
      $composableBuilder(column: $table.name, builder: (column) => column);

  GeneratedColumn<String> get email =>
      $composableBuilder(column: $table.email, builder: (column) => column);

  GeneratedColumn<int> get age =>
      $composableBuilder(column: $table.age, builder: (column) => column);

  GeneratedColumn<double> get score =>
      $composableBuilder(column: $table.score, builder: (column) => column);

  GeneratedColumn<int> get createdAt =>
      $composableBuilder(column: $table.createdAt, builder: (column) => column);
}

class $$DriftRecordsTableTableManager
    extends
        RootTableManager<
          _$BenchmarkDatabase,
          $DriftRecordsTable,
          DriftRecord,
          $$DriftRecordsTableFilterComposer,
          $$DriftRecordsTableOrderingComposer,
          $$DriftRecordsTableAnnotationComposer,
          $$DriftRecordsTableCreateCompanionBuilder,
          $$DriftRecordsTableUpdateCompanionBuilder,
          (
            DriftRecord,
            BaseReferences<
              _$BenchmarkDatabase,
              $DriftRecordsTable,
              DriftRecord
            >,
          ),
          DriftRecord,
          PrefetchHooks Function()
        > {
  $$DriftRecordsTableTableManager(
    _$BenchmarkDatabase db,
    $DriftRecordsTable table,
  ) : super(
        TableManagerState(
          db: db,
          table: table,
          createFilteringComposer: () =>
              $$DriftRecordsTableFilterComposer($db: db, $table: table),
          createOrderingComposer: () =>
              $$DriftRecordsTableOrderingComposer($db: db, $table: table),
          createComputedFieldComposer: () =>
              $$DriftRecordsTableAnnotationComposer($db: db, $table: table),
          updateCompanionCallback:
              ({
                Value<int> id = const Value.absent(),
                Value<String> name = const Value.absent(),
                Value<String> email = const Value.absent(),
                Value<int> age = const Value.absent(),
                Value<double> score = const Value.absent(),
                Value<int> createdAt = const Value.absent(),
              }) => DriftRecordsCompanion(
                id: id,
                name: name,
                email: email,
                age: age,
                score: score,
                createdAt: createdAt,
              ),
          createCompanionCallback:
              ({
                Value<int> id = const Value.absent(),
                required String name,
                required String email,
                required int age,
                required double score,
                required int createdAt,
              }) => DriftRecordsCompanion.insert(
                id: id,
                name: name,
                email: email,
                age: age,
                score: score,
                createdAt: createdAt,
              ),
          withReferenceMapper: (p0) => p0
              .map((e) => (e.readTable(table), BaseReferences(db, table, e)))
              .toList(),
          prefetchHooksCallback: null,
        ),
      );
}

typedef $$DriftRecordsTableProcessedTableManager =
    ProcessedTableManager<
      _$BenchmarkDatabase,
      $DriftRecordsTable,
      DriftRecord,
      $$DriftRecordsTableFilterComposer,
      $$DriftRecordsTableOrderingComposer,
      $$DriftRecordsTableAnnotationComposer,
      $$DriftRecordsTableCreateCompanionBuilder,
      $$DriftRecordsTableUpdateCompanionBuilder,
      (
        DriftRecord,
        BaseReferences<_$BenchmarkDatabase, $DriftRecordsTable, DriftRecord>,
      ),
      DriftRecord,
      PrefetchHooks Function()
    >;

class $BenchmarkDatabaseManager {
  final _$BenchmarkDatabase _db;
  $BenchmarkDatabaseManager(this._db);
  $$DriftRecordsTableTableManager get driftRecords =>
      $$DriftRecordsTableTableManager(_db, _db.driftRecords);
}
