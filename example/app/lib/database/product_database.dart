import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_example/models/product_models.dart';
import 'package:nodedb_example/models/user_models.dart';

/// Product domain database — manages products, categories, and orders.
///
/// Participates in the mesh with `databaseName: 'products'`.
class ProductDatabase {
  late final NodeDB db;
  late final ProductDao products;
  late final CategoryDao categories;
  late final OrderDao orders;
  late final SearchResultDao searchResults;

  void init(String baseDir, DatabaseMesh mesh) {
    final dir = Directory('$baseDir/products');
    if (!dir.existsSync()) dir.createSync(recursive: true);

    db = NodeDB.open(
      directory: dir.path,
      databaseName: 'products',
      mesh: mesh,
      provenanceEnabled: true,
    );

    products = ProductDao(db.nosql, db.provenance, db.notifier);
    categories = CategoryDao(db.nosql, db.provenance, db.notifier);
    orders = OrderDao(db.nosql, db.provenance, db.notifier);
    searchResults = SearchResultDao(db.nosql, db.provenance, db.notifier);
  }

  /// Seed sample products and categories if empty.
  void seedIfEmpty(List<User> existingUsers) {
    if (products.count() > 0) return;

    categories.createAll([
      Category(name: 'Electronics'),
      Category(name: 'Books'),
      Category(name: 'Clothing'),
    ]);

    final creatorId = existingUsers.isNotEmpty ? existingUsers.first.id : '';

    products.createAll([
      Product(
        name: 'Laptop Pro',
        description: 'High-performance laptop for developers',
        price: 1299.99,
        category: 'Electronics',
        createdBy: creatorId,
        productMetadata: ProductMetadata(color: 'silver', weight: 1800),
      ),
      Product(
        name: 'NodeDB Handbook',
        description: 'Complete guide to embedded multi-engine databases',
        price: 39.99,
        category: 'Books',
        createdBy: creatorId,
        productMetadata: ProductMetadata(color: 'white', weight: 350, material: 'paper'),
      ),
      Product(
        name: 'Developer T-Shirt',
        description: 'Comfortable cotton t-shirt with NodeDB logo',
        price: 24.99,
        category: 'Clothing',
        createdBy: creatorId,
        productMetadata: ProductMetadata(color: 'black', weight: 200, material: 'cotton'),
      ),
    ]);
  }
}
