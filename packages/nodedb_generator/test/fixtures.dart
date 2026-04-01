import 'package:nodedb_generator/src/model_info.dart';

/// Basic @collection model: User with id, name, email (indexed+unique), age, createdAt (nullable DateTime).
final userModel = ModelInfo(
  className: 'User',
  collectionName: 'users',
  type: ModelType.collection,
  schema: 'public',
  singleton: false,
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(
      name: 'email',
      dartType: 'String',
      index: IndexInfo(unique: true),
    ),
    FieldInfo(name: 'age', dartType: 'int'),
    FieldInfo(name: 'createdAt', dartType: 'DateTime', isNullable: true),
  ],
  indexes: [IndexInfo(unique: true)],
);

/// Complex @collection model with list, nullable, and various field types.
final articleModel = ModelInfo(
  className: 'Article',
  collectionName: 'articles',
  type: ModelType.collection,
  schema: 'public',
  singleton: false,
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(
      name: 'title',
      dartType: 'String',
      index: IndexInfo(unique: true),
    ),
    FieldInfo(name: 'body', dartType: 'String'),
    FieldInfo(name: 'publishedAt', dartType: 'DateTime', isNullable: true),
    FieldInfo(
      name: 'tags',
      dartType: 'List<String>',
      isList: true,
      listElementType: 'String',
    ),
    FieldInfo(name: 'rating', dartType: 'double'),
    FieldInfo(name: 'draft', dartType: 'bool'),
  ],
  indexes: [IndexInfo(unique: true)],
);

/// @node model for graph.
final personNode = ModelInfo(
  className: 'Person',
  collectionName: 'people',
  type: ModelType.node,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(name: 'age', dartType: 'int'),
  ],
  indexes: [],
);

/// @edge model for graph edges.
final knowsEdge = ModelInfo(
  className: 'Knows',
  collectionName: 'knows',
  type: ModelType.edge,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'since', dartType: 'DateTime'),
    FieldInfo(name: 'strength', dartType: 'double'),
  ],
  indexes: [],
  edgeInfo: EdgeInfo(fromType: 'Person', toType: 'Person'),
);

/// @collection with generateDao: false.
final noDaoModel = ModelInfo(
  className: 'InternalLog',
  collectionName: 'internal_logs',
  type: ModelType.collection,
  generateDao: false,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'message', dartType: 'String'),
    FieldInfo(name: 'level', dartType: 'int'),
  ],
  indexes: [],
);

/// @preferences model.
final appPrefsModel = ModelInfo(
  className: 'AppPrefs',
  collectionName: 'app_prefs',
  type: ModelType.preferences,
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'locale', dartType: 'String'),
    FieldInfo(name: 'fontSize', dartType: 'int'),
    FieldInfo(name: 'darkMode', dartType: 'bool'),
  ],
  indexes: [],
);

/// Singleton @collection.
final settingsModel = ModelInfo(
  className: 'Settings',
  collectionName: 'settings',
  type: ModelType.collection,
  singleton: true,
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'theme', dartType: 'String'),
    FieldInfo(name: 'fontSize', dartType: 'int'),
  ],
  indexes: [],
);

/// String-id @collection model: User with UUID v7 id.
final userModelStringId = ModelInfo(
  className: 'User',
  collectionName: 'users',
  type: ModelType.collection,
  schema: 'public',
  singleton: false,
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'String'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(
      name: 'email',
      dartType: 'String',
      index: IndexInfo(unique: true),
    ),
    FieldInfo(name: 'age', dartType: 'int'),
    FieldInfo(name: 'createdAt', dartType: 'DateTime', isNullable: true),
  ],
  indexes: [IndexInfo(unique: true)],
);

/// @collection with @Trimmable annotation.
final trimmableModel = ModelInfo(
  className: 'LogEntry',
  collectionName: 'log_entries',
  type: ModelType.collection,
  generateDao: true,
  trimmable: true,
  trimPolicy: 'age:30d',
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'message', dartType: 'String'),
    FieldInfo(name: 'level', dartType: 'int'),
    FieldInfo(name: 'timestamp', dartType: 'DateTime'),
  ],
  indexes: [],
);

/// @collection with @NeverTrim annotation.
final neverTrimModel = ModelInfo(
  className: 'AuditRecord',
  collectionName: 'audit_records',
  type: ModelType.collection,
  generateDao: true,
  neverTrim: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'action', dartType: 'String'),
    FieldInfo(name: 'actor', dartType: 'String'),
  ],
  indexes: [],
);

/// @collection with JSONB Map and List fields.
final productModelWithJsonb = ModelInfo(
  className: 'Product',
  collectionName: 'products',
  type: ModelType.collection,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'String'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(name: 'price', dartType: 'double'),
    FieldInfo(
      name: 'metadata',
      dartType: 'Map<String, dynamic>',
      isJsonb: true,
    ),
    FieldInfo(
      name: 'categories',
      dartType: 'List<String>',
      isList: true,
      listElementType: 'String',
    ),
  ],
  indexes: [],
);

/// @NodeDBView model: cross-collection view.
final userProductView = ModelInfo(
  className: 'UserProductView',
  collectionName: 'user_product_views',
  type: ModelType.view,
  generateDao: true,
  fields: [
    FieldInfo(name: 'userName', dartType: 'String'),
    FieldInfo(name: 'productName', dartType: 'String'),
    FieldInfo(name: 'price', dartType: 'double'),
  ],
  indexes: [],
  viewInfo: ViewInfo(
    sources: [
      ViewSourceInfo(collection: 'users'),
      ViewSourceInfo(collection: 'products', database: 'warehouse'),
    ],
    strategy: 'union',
  ),
);

/// @collection with enhanced @Jsonb (schema + identifier) fields.
final productWithEnhancedJsonb = ModelInfo(
  className: 'Product',
  collectionName: 'products',
  type: ModelType.collection,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'String'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(
      name: 'attributes',
      dartType: 'Map<String, dynamic>',
      isJsonb: true,
      jsonbSchema: '{"type": "object"}',
      jsonbIdentifier: 'sku',
    ),
  ],
  indexes: [],
);

/// @collection with a @JsonModel typed field.
final productWithJsonModel = ModelInfo(
  className: 'Product',
  collectionName: 'products',
  type: ModelType.collection,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'String'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(
      name: 'metadata',
      dartType: 'ProductMetadata',
      isJsonModel: true,
      jsonModelType: 'ProductMetadata',
    ),
  ],
  indexes: [],
);

/// @collection with a nullable @JsonModel typed field.
final productWithNullableJsonModel = ModelInfo(
  className: 'Product',
  collectionName: 'products',
  type: ModelType.collection,
  schema: 'public',
  generateDao: true,
  fields: [
    FieldInfo(name: 'id', dartType: 'String'),
    FieldInfo(name: 'name', dartType: 'String'),
    FieldInfo(
      name: 'metadata',
      dartType: 'ProductMetadata',
      isJsonModel: true,
      jsonModelType: 'ProductMetadata',
      isNullable: true,
    ),
  ],
  indexes: [],
);

/// Standalone @JsonModel class (serialization only, no DAO).
final jsonModelFixture = ModelInfo(
  className: 'ProductMetadata',
  collectionName: 'packages/example/ProductMetadata',
  type: ModelType.collection,
  generateDao: false,
  fields: [
    FieldInfo(name: 'color', dartType: 'String'),
    FieldInfo(name: 'weight', dartType: 'int'),
    FieldInfo(name: 'material', dartType: 'String', isNullable: true),
  ],
  indexes: [],
);

/// @JsonModel with a nested @JsonModel field (explicitToJson-like).
final nestedJsonModelFixture = ModelInfo(
  className: 'ProductDetails',
  collectionName: 'packages/example/ProductDetails',
  type: ModelType.collection,
  generateDao: false,
  fields: [
    FieldInfo(name: 'sku', dartType: 'String'),
    FieldInfo(
      name: 'metadata',
      dartType: 'ProductMetadata',
      isJsonModel: true,
      jsonModelType: 'ProductMetadata',
    ),
    FieldInfo(
      name: 'extra',
      dartType: 'ExtraInfo',
      isJsonModel: true,
      jsonModelType: 'ExtraInfo',
      isNullable: true,
    ),
  ],
  indexes: [],
);

/// @collection with @Trigger annotations.
final triggeredModel = ModelInfo(
  className: 'Order',
  collectionName: 'orders',
  type: ModelType.collection,
  generateDao: true,
  triggers: [
    TriggerInfo(event: 'insert', timing: 'after', name: 'on_order_created'),
    TriggerInfo(event: 'update', timing: 'before'),
  ],
  fields: [
    FieldInfo(name: 'id', dartType: 'int'),
    FieldInfo(name: 'total', dartType: 'double'),
    FieldInfo(name: 'status', dartType: 'String'),
  ],
  indexes: [],
);
