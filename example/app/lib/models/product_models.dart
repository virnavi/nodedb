/// Product domain models — Products DB.
///
/// Models: Product, Category, Order, ProductMetadata (JsonModel)
/// Database: meshName='nodedb-example', databaseName='products'
library;

import 'package:nodedb/nodedb.dart';

part 'product_models.g.dart';

// ─────────────────────────────────────────────────────────────────
// @JsonModel — ProductMetadata (typed JSONB, stored as Map)
// ─────────────────────────────────────────────────────────────────

@JsonModel()
class ProductMetadata {
  late String color;
  late int weight;
  String? material;

  ProductMetadata({
    required this.color,
    required this.weight,
    this.material,
  });
}

// ─────────────────────────────────────────────────────────────────
// @collection — Product
// ─────────────────────────────────────────────────────────────────

@collection
class Product {
  String id = '';
  late String name;
  late String description;
  late double price;
  late String category;
  late String createdBy; // user id who created this product
  DateTime? createdAt;
  @Jsonb(schema: '{"type": "object"}', identifier: 'sku')
  late Map<String, dynamic> extras; // JSONB field with schema + identifier
  late ProductMetadata productMetadata; // typed JSONB via @JsonModel
  late List<String> tags; // array field for product tags

  Product({
    this.id = '',
    required this.name,
    required this.description,
    required this.price,
    required this.category,
    required this.createdBy,
    this.createdAt,
    this.extras = const {},
    required this.productMetadata,
    this.tags = const [],
  });
}

// ─────────────────────────────────────────────────────────────────
// @collection — Category
// ─────────────────────────────────────────────────────────────────

@collection
class Category {
  String id = '';
  @Index(unique: true)
  late String name;

  Category({
    this.id = '',
    required this.name,
  });
}

// ─────────────────────────────────────────────────────────────────
// @collection — Order
// ─────────────────────────────────────────────────────────────────

@collection
class Order {
  String id = '';
  late String productId;
  late String buyerId;
  late String status; // pending, confirmed, shipped, delivered
  DateTime? createdAt;

  Order({
    this.id = '',
    required this.productId,
    required this.buyerId,
    this.status = 'pending',
    this.createdAt,
  });
}

// ─────────────────────────────────────────────────────────────────
// @collection — SearchResult (demonstrates cache TTL)
// ─────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────
// @NodeDBView — UserProductView (cross-database view)
// ─────────────────────────────────────────────────────────────────

@NodeDBView(sources: [
  ViewSource(collection: 'users', database: 'users'),
  ViewSource(collection: 'products'),
])
class UserProductView {
  late String name;
  late String description;
  late double price;

  UserProductView({
    required this.name,
    required this.description,
    this.price = 0.0,
  });
}

// ─────────────────────────────────────────────────────────────────
// @collection — SearchResult (demonstrates cache TTL)
// ─────────────────────────────────────────────────────────────────

@collection
@Trimmable()
class SearchResult {
  String id = '';
  late String query;
  late String resultJson;
  DateTime? cachedAt;

  SearchResult({
    this.id = '',
    required this.query,
    required this.resultJson,
    this.cachedAt,
  });
}
