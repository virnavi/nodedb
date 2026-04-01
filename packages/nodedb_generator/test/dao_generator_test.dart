import 'package:nodedb_generator/src/dao_generator.dart';
import 'package:nodedb_generator/src/dao_registry_generator.dart';
import 'package:test/test.dart';

import 'fixtures.dart';

void main() {
  group('generateDao — collection', () {
    late String output;

    setUp(() {
      output = generateDao(userModel);
    });

    test('generates base DAO class', () {
      expect(output, contains('abstract class UserDaoBase'));
    });

    test('has correct collection name', () {
      expect(output, contains("String get collectionName => 'users'"));
    });

    test('generates findById', () {
      expect(output, contains('User? findById(int id)'));
    });

    test('generates findAll with filter query', () {
      expect(output, contains('List<User> findAll([FilterQuery<User>? query])'));
    });

    test('generates findFirst with builder pattern', () {
      expect(output, contains('User? findFirst(FilterQuery<User> Function(FilterQuery<User>) filter)'));
    });

    test('generates create method', () {
      expect(output, contains('void create(User item)'));
    });

    test('generates createAll method', () {
      expect(output, contains('void createAll(List<User> items)'));
    });

    test('generates save with optional id', () {
      expect(output, contains('void save(User item, {int? id})'));
    });

    test('generates deleteById', () {
      expect(output, contains('bool deleteById(int id)'));
    });

    test('generates count', () {
      expect(output, contains('int count()'));
    });

    test('generates findPage with pagination', () {
      expect(output, contains('List<User> findPage({required int limit, int offset = 0})'));
    });

    test('generates exists helper', () {
      expect(output, contains('bool exists(FilterQuery<User> Function(FilterQuery<User>) filter)'));
    });

    test('generates updateById with modifier function', () {
      expect(output, contains('User? updateById(int id, User Function(User current) modifier)'));
    });

    test('references serialization functions', () {
      expect(output, contains('_\$UserFromMap'));
      expect(output, contains('_\$UserToMap'));
    });

    test('uses NoSqlEngine', () {
      expect(output, contains('NoSqlEngine get _engine'));
    });
  });

  group('generateDao — node', () {
    late String output;

    setUp(() {
      output = generateDao(personNode);
    });

    test('generates node DAO base', () {
      expect(output, contains('abstract class PersonNodeDaoBase'));
    });

    test('generates addNode', () {
      expect(output, contains('Person addNode(Person item)'));
    });

    test('generates getNode', () {
      expect(output, contains('Person? getNode(int id)'));
    });

    test('generates deleteNode with behaviour', () {
      expect(output, contains("void deleteNode(int id, {String behaviour = 'detach'})"));
    });

    test('generates bfs/dfs with correct return type', () {
      expect(output, contains('Map<String, List<int>> bfs(int startId'));
      expect(output, contains('Map<String, List<int>> dfs(int startId'));
    });

    test('generates edgesFrom/edgesTo', () {
      expect(output, contains('List<GraphEdge> edgesFrom(int nodeId)'));
      expect(output, contains('List<GraphEdge> edgesTo(int nodeId)'));
    });

    test('uses GraphEngine', () {
      expect(output, contains('GraphEngine get _graphEngine'));
    });

    test('filters allNodes by label', () {
      expect(output, contains("n.label == 'people'"));
    });
  });

  group('generateDao — edge', () {
    late String output;

    setUp(() {
      output = generateDao(knowsEdge);
    });

    test('generates edge DAO base', () {
      expect(output, contains('abstract class KnowsEdgeDaoBase'));
    });

    test('generates addEdge with weight', () {
      expect(output, contains('GraphEdge addEdge(int sourceId, int targetId, Knows data, {double weight = 1.0})'));
    });

    test('generates getEdge', () {
      expect(output, contains('GraphEdge? getEdge(int id)'));
    });

    test('generates deleteEdge', () {
      expect(output, contains('void deleteEdge(int id)'));
    });

    test('filters edges by label', () {
      expect(output, contains("e.label == 'knows'"));
    });
  });

  group('generateDao — singleton', () {
    late String output;

    setUp(() {
      output = generateDao(settingsModel);
    });

    test('generates singleton DAO base class', () {
      expect(output, contains('abstract class SettingsDaoBase'));
    });

    test('has correct collection name', () {
      expect(output, contains("String get collectionName => 'settings'"));
    });

    test('generates init method', () {
      expect(output, contains('Settings init(Settings defaults)'));
    });

    test('generates get method', () {
      expect(output, contains('Settings get()'));
    });

    test('generates put method', () {
      expect(output, contains('Settings put(Settings item)'));
    });

    test('generates update with modifier function', () {
      expect(output, contains('Settings update(Settings Function(Settings current) modifier)'));
    });

    test('generates reset method', () {
      expect(output, contains('Settings reset()'));
    });

    test('generates isSingleton getter', () {
      expect(output, contains('bool get isSingleton'));
    });

    test('references serialization functions', () {
      expect(output, contains('_\$SettingsFromMap'));
      expect(output, contains('_\$SettingsToMap'));
    });

    test('uses NoSqlEngine', () {
      expect(output, contains('NoSqlEngine get _engine'));
    });

    test('delegates to singleton engine methods', () {
      expect(output, contains('_engine.singletonCreate'));
      expect(output, contains('_engine.singletonGet'));
      expect(output, contains('_engine.singletonPut'));
      expect(output, contains('_engine.singletonReset'));
      expect(output, contains('_engine.isSingleton'));
    });

    test('does NOT generate CRUD methods', () {
      expect(output, isNot(contains('findById')));
      expect(output, isNot(contains('findAll')));
      expect(output, isNot(contains('create(')));
      expect(output, isNot(contains('deleteById')));
      expect(output, isNot(contains('count()')));
    });

    test('has singleton comment header', () {
      expect(output, contains('Singleton DAO'));
    });
  });

  group('generateConcreteDao', () {
    test('collection DAO extends base', () {
      final output = generateConcreteDao(userModel);
      expect(output, contains('class UserDao extends UserDaoBase'));
      expect(output, contains('final NoSqlEngine _engine'));
    });

    test('node DAO extends base', () {
      final output = generateConcreteDao(personNode);
      expect(output, contains('class PersonDao extends PersonNodeDaoBase'));
      expect(output, contains('final GraphEngine _graphEngine'));
    });

    test('edge DAO extends base', () {
      final output = generateConcreteDao(knowsEdge);
      expect(output, contains('class KnowsDao extends KnowsEdgeDaoBase'));
      expect(output, contains('final GraphEngine _graphEngine'));
    });

    test('collection concrete DAO accepts optional ProvenanceEngine', () {
      final output = generateConcreteDao(userModel);
      expect(output, contains('ProvenanceEngine? _provenanceEngine'));
      expect(output, contains('this._provenanceEngine'));
      expect(output, contains('CollectionNotifier? _notifier'));
      expect(output, contains('this._notifier'));
    });
  });

  group('generateDao — provenance support', () {
    late String output;

    setUp(() {
      output = generateDao(userModel);
    });

    test('base DAO has _provenanceEngine getter', () {
      expect(output, contains('ProvenanceEngine? get _provenanceEngine'));
    });

    test('generates findAllWithProvenance method', () {
      expect(output, contains('findAllWithProvenance'));
      expect(output, contains('WithProvenance<User>'));
    });

    test('generates findWhereWithProvenance method', () {
      expect(output, contains('findWhereWithProvenance'));
    });

    test('findAllWithProvenance uses getForRecord', () {
      expect(output, contains('getForRecord(collectionName, doc.id)'));
    });

    test('generates findByIdWithProvenance method', () {
      expect(output, contains('WithProvenance<User>? findByIdWithProvenance(int id)'));
    });

    test('findByIdWithProvenance returns null for missing doc', () {
      expect(output, contains('if (doc == null) return null'));
    });

    test('node DAO does not have provenance methods', () {
      final nodeOutput = generateDao(personNode);
      expect(nodeOutput, isNot(contains('findAllWithProvenance')));
    });

    test('singleton DAO does not have provenance methods', () {
      final singletonOutput = generateDao(settingsModel);
      expect(singletonOutput, isNot(contains('findAllWithProvenance')));
    });

    test('singleton DAO has _provenanceEngine getter', () {
      final singletonOutput = generateDao(settingsModel);
      expect(singletonOutput, contains('ProvenanceEngine? get _provenanceEngine'));
    });

    test('singleton DAO generates getWithProvenance', () {
      final singletonOutput = generateDao(settingsModel);
      expect(singletonOutput, contains('WithProvenance<Settings> getWithProvenance()'));
    });
  });

  group('generateDao — trimmable', () {
    late String output;

    setUp(() {
      output = generateDao(trimmableModel);
    });

    test('generates isTrimmable getter', () {
      expect(output, contains('bool get isTrimmable => true'));
    });

    test('generates trim method', () {
      expect(output, contains('TrimReport trim(TrimPolicy policy, {bool dryRun = false})'));
    });

    test('trim delegates to engine', () {
      expect(output, contains('_engine.trim(collectionName, policy, dryRun: dryRun)'));
    });

    test('generates recommendTrim method', () {
      expect(output, contains('TrimRecommendation recommendTrim(TrimPolicy policy)'));
    });

    test('generates setTrimPolicy method', () {
      expect(output, contains('void setTrimPolicy(TrimPolicy policy)'));
    });

    test('generates effectiveTrimPolicy getter', () {
      expect(output, contains('TrimPolicy? get effectiveTrimPolicy'));
    });

    test('non-trimmable model does not have trim methods', () {
      final regularOutput = generateDao(userModel);
      expect(regularOutput, isNot(contains('isTrimmable')));
      expect(regularOutput, isNot(contains('TrimReport')));
      expect(regularOutput, isNot(contains('setTrimPolicy')));
    });
  });

  group('generateDao — neverTrim', () {
    test('generates isNeverTrim getter', () {
      final output = generateDao(neverTrimModel);
      expect(output, contains('bool get isNeverTrim => true'));
    });

    test('regular model does not have isNeverTrim', () {
      final output = generateDao(userModel);
      expect(output, isNot(contains('isNeverTrim')));
    });

    test('neverTrim model does not have trim methods', () {
      final output = generateDao(neverTrimModel);
      expect(output, isNot(contains('TrimReport')));
      expect(output, isNot(contains('setTrimPolicy')));
    });
  });

  group('generateDao — triggers', () {
    late String output;

    setUp(() {
      output = generateDao(triggeredModel);
    });

    test('generates registerDeclaredTriggers method', () {
      expect(output, contains('List<int> registerDeclaredTriggers()'));
    });

    test('registers correct events', () {
      expect(output, contains("event: 'insert'"));
      expect(output, contains("event: 'update'"));
    });

    test('registers correct timing', () {
      expect(output, contains("timing: 'after'"));
      expect(output, contains("timing: 'before'"));
    });

    test('registers named triggers', () {
      expect(output, contains("name: 'on_order_created'"));
    });

    test('returns list of trigger IDs', () {
      expect(output, contains('return ids'));
    });

    test('model without triggers has no registration method', () {
      final output = generateDao(userModel);
      expect(output, isNot(contains('registerDeclaredTriggers')));
    });
  });

  group('generateDao — String id collection', () {
    late String output;

    setUp(() {
      output = generateDao(userModelStringId);
    });

    test('generates findById with String parameter', () {
      expect(output, contains('User? findById(String id)'));
    });

    test('generates _findDocumentById helper', () {
      expect(output, contains('Document? _findDocumentById(String id)'));
    });

    test('uses filter query for id lookup', () {
      expect(output, contains("'EqualTo': {'field': 'id', 'value': id}"));
    });

    test('does not use direct engine get for findById', () {
      expect(output, isNot(contains('_engine.get(collectionName, id)')));
    });

    test('generates create with UUID auto-generation', () {
      expect(output, contains('generateNodeDbId()'));
    });

    test('create auto-assigns UUID for empty id', () {
      expect(output, contains("map['id'] = generateNodeDbId()"));
    });

    test('generates save without optional int id parameter', () {
      expect(output, contains('void save(User item)'));
      expect(output, isNot(contains('void save(User item, {int? id})')));
    });

    test('save does upsert via _findDocumentById', () {
      expect(output, contains('final existing = _findDocumentById('));
      expect(output, contains('id: existing?.id'));
    });

    test('generates deleteById with String parameter', () {
      expect(output, contains('bool deleteById(String id)'));
    });

    test('deleteById returns false when not found', () {
      expect(output, contains('if (doc == null) return false'));
    });

    test('generates deleteAllById with String list', () {
      expect(output, contains('void deleteAllById(List<String> ids)'));
    });

    test('generates updateById with String parameter', () {
      expect(output, contains('User? updateById(String id, User Function(User current) modifier)'));
    });

    test('updateById preserves the UUID', () {
      expect(output, contains("map['id'] = id"));
    });

    test('generates findByIdWithProvenance with String parameter', () {
      expect(output, contains('WithProvenance<User>? findByIdWithProvenance(String id)'));
    });

    test('deleteWhere actually deletes documents', () {
      expect(output, contains('_findDocumentById(item.id)'));
      expect(output, contains('WriteOp.delete(collectionName, id: doc.id)'));
    });

    test('still has shared methods unchanged', () {
      expect(output, contains('List<User> findAll('));
      expect(output, contains('User? findFirst('));
      expect(output, contains('List<User> findWhere('));
      expect(output, contains('int count()'));
      expect(output, contains('List<User> findPage('));
    });
  });

  group('generateDao — int id backward compatibility', () {
    test('int-id model generates findById with int', () {
      final output = generateDao(userModel);
      expect(output, contains('User? findById(int id)'));
    });

    test('int-id model uses direct engine get', () {
      final output = generateDao(userModel);
      expect(output, contains('_engine.get(collectionName, id)'));
    });

    test('int-id model does not use generateNodeDbId', () {
      final output = generateDao(userModel);
      expect(output, isNot(contains('generateNodeDbId')));
    });

    test('int-id model has save with optional int id', () {
      final output = generateDao(userModel);
      expect(output, contains('void save(User item, {int? id})'));
    });

    test('int-id model deleteById takes int', () {
      final output = generateDao(userModel);
      expect(output, contains('bool deleteById(int id)'));
    });
  });

  group('generateDao — FQN collection names', () {
    test('collection DAO has schemaName constant', () {
      final output = generateDao(userModelStringId);
      expect(output, contains("static const schemaName = 'public'"));
    });

    test('collection DAO has qualifiedName getter', () {
      final output = generateDao(userModelStringId);
      expect(output, contains('String get qualifiedName'));
    });

    test('collection DAO has _databaseName getter', () {
      final output = generateDao(userModelStringId);
      expect(output, contains('String? get _databaseName => null'));
    });

    test('singleton DAO has schemaName constant', () {
      final output = generateDao(settingsModel);
      expect(output, contains("static const schemaName = 'public'"));
    });

    test('singleton DAO has qualifiedName getter', () {
      final output = generateDao(settingsModel);
      expect(output, contains('String get qualifiedName'));
    });

    test('concrete DAO has _databaseName field', () {
      final output = generateConcreteDao(userModelStringId);
      expect(output, contains('final String? _databaseName'));
    });

    test('concrete DAO accepts databaseName in constructor', () {
      final output = generateConcreteDao(userModelStringId);
      expect(output, contains('this._databaseName'));
    });
  });
}
