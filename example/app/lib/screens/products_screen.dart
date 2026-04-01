import 'dart:async';

import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_example/database/mesh_service.dart';
import 'package:nodedb_example/models/product_models.dart';

/// Products screen — list products, create new, and **federated search**.
///
/// The federated search is the key demo: searching for a product queries
/// the local database first, then reaches out to mesh peers.
class ProductsScreen extends StatefulWidget {
  final MeshService mesh;
  const ProductsScreen({super.key, required this.mesh});

  @override
  State<ProductsScreen> createState() => _ProductsScreenState();
}

class _ProductsScreenState extends State<ProductsScreen> {
  List<Product> _products = [];
  List<FederatedResult<Document>> _federatedResults = [];
  final _searchCtrl = TextEditingController();
  bool _searchingMesh = false;
  StreamSubscription<List<Product>>? _watchSub;

  @override
  void initState() {
    super.initState();
    _watchSub = widget.mesh.productDb.products.watchAll().listen((products) {
      setState(() => _products = products);
    });
  }

  @override
  void dispose() {
    _watchSub?.cancel();
    _searchCtrl.dispose();
    super.dispose();
  }

  void _searchLocal(String query) {
    _watchSub?.cancel();
    if (query.isEmpty) {
      _watchSub = widget.mesh.productDb.products.watchAll().listen((products) {
        setState(() {
          _products = products;
          _federatedResults = [];
        });
      });
      return;
    }
    setState(() {
      _products = widget.mesh.productDb.products
          .findWhere((q) => q.nameContains(query));
      _federatedResults = [];
    });
  }

  void _searchMesh(String query) {
    if (query.isEmpty) return;
    setState(() => _searchingMesh = true);

    final results = widget.mesh.searchProductsFederated(query);

    setState(() {
      _federatedResults = results;
      _searchingMesh = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _searchCtrl,
                    decoration: const InputDecoration(
                      hintText: 'Search products...',
                      prefixIcon: Icon(Icons.search),
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    onChanged: _searchLocal,
                  ),
                ),
                const SizedBox(width: 8),
                FilledButton.icon(
                  onPressed: _searchingMesh
                      ? null
                      : () => _searchMesh(_searchCtrl.text),
                  icon: const Icon(Icons.cloud_sync),
                  label: const Text('Mesh'),
                ),
              ],
            ),
          ),

          // Local results
          if (_products.isNotEmpty) ...[
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  Icon(Icons.storage, size: 16, color: theme.colorScheme.primary),
                  const SizedBox(width: 4),
                  Text('Local Results (${_products.length})',
                      style: theme.textTheme.titleSmall),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                itemCount: _products.length,
                itemBuilder: (context, index) =>
                    _ProductTile(product: _products[index], source: 'local'),
              ),
            ),
          ],

          // Federated results
          if (_federatedResults.isNotEmpty) ...[
            const Divider(),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  Icon(Icons.cloud, size: 16, color: theme.colorScheme.tertiary),
                  const SizedBox(width: 4),
                  Text('Mesh Results (${_federatedResults.length})',
                      style: theme.textTheme.titleSmall),
                ],
              ),
            ),
            Expanded(
              child: ListView.builder(
                itemCount: _federatedResults.length,
                itemBuilder: (context, index) {
                  final result = _federatedResults[index];
                  final data = result.data.data;
                  return ListTile(
                    leading: CircleAvatar(
                      backgroundColor: theme.colorScheme.tertiaryContainer,
                      child: const Icon(Icons.cloud_download),
                    ),
                    title: Text(data['name']?.toString() ?? ''),
                    subtitle: Text(
                      'From: ${result.sourcePeerId} | '
                      '\$${data['price'] ?? 0}',
                    ),
                    trailing: Chip(
                      label: Text(result.sourcePeerId == 'local'
                          ? 'Local'
                          : 'Peer'),
                      backgroundColor: result.sourcePeerId == 'local'
                          ? theme.colorScheme.primaryContainer
                          : theme.colorScheme.tertiaryContainer,
                    ),
                  );
                },
              ),
            ),
          ],

          // Empty state
          if (_products.isEmpty && _federatedResults.isEmpty)
            const Expanded(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.inventory_2_outlined,
                        size: 48, color: Colors.grey),
                    SizedBox(height: 8),
                    Text('No products found'),
                    SizedBox(height: 4),
                    Text(
                      'Try searching the mesh to find products\non other devices',
                      textAlign: TextAlign.center,
                      style: TextStyle(color: Colors.grey),
                    ),
                  ],
                ),
              ),
            ),

          if (_searchingMesh)
            const Padding(
              padding: EdgeInsets.all(16),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                  SizedBox(width: 8),
                  Text('Querying mesh peers...'),
                ],
              ),
            ),
        ],
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _showCreateDialog(),
        child: const Icon(Icons.add),
      ),
    );
  }

  void _showCreateDialog() {
    final nameCtrl = TextEditingController();
    final descCtrl = TextEditingController();
    final priceCtrl = TextEditingController();
    String selectedCategory = 'Electronics';

    final cats =
        widget.mesh.productDb.categories.findAll().map((c) => c.name).toList();
    if (cats.isEmpty) cats.addAll(['Electronics', 'Books', 'Clothing']);

    // Use first user as creator
    final users = widget.mesh.userDb.users.findAll();
    final creatorId = users.isNotEmpty ? users.first.id : '';

    showDialog(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: const Text('New Product'),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TextField(
                  controller: nameCtrl,
                  decoration: const InputDecoration(labelText: 'Name'),
                ),
                TextField(
                  controller: descCtrl,
                  decoration: const InputDecoration(labelText: 'Description'),
                  maxLines: 2,
                ),
                TextField(
                  controller: priceCtrl,
                  decoration: const InputDecoration(labelText: 'Price'),
                  keyboardType: TextInputType.number,
                ),
                const SizedBox(height: 8),
                DropdownButtonFormField<String>(
                  initialValue: selectedCategory,
                  decoration: const InputDecoration(labelText: 'Category'),
                  items: cats
                      .map((c) =>
                          DropdownMenuItem(value: c, child: Text(c)))
                      .toList(),
                  onChanged: (v) {
                    if (v != null) {
                      setDialogState(() => selectedCategory = v);
                    }
                  },
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                final price = double.tryParse(priceCtrl.text) ?? 0;
                if (nameCtrl.text.isNotEmpty) {
                  widget.mesh.productDb.products.create(Product(
                    name: nameCtrl.text,
                    description: descCtrl.text,
                    price: price,
                    category: selectedCategory,
                    createdBy: creatorId,
                    productMetadata: ProductMetadata(color: 'default', weight: 0),
                  ));
                  Navigator.pop(ctx);
                }
              },
              child: const Text('Create'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ProductTile extends StatelessWidget {
  final Product product;
  final String source;
  const _ProductTile({required this.product, required this.source});

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: CircleAvatar(child: Text(product.name[0])),
      title: Text(product.name),
      subtitle: Text(
        '${product.category} | \$${product.price.toStringAsFixed(2)}',
      ),
      trailing: Text(product.description,
          style: Theme.of(context).textTheme.bodySmall,
          overflow: TextOverflow.ellipsis),
    );
  }
}
