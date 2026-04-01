import 'dart:async';

import 'package:flutter/material.dart';
import 'package:nodedb_example/database/mesh_service.dart';
import 'package:nodedb_example/models/product_models.dart';

/// Orders screen — create and manage orders referencing products and users.
class OrdersScreen extends StatefulWidget {
  final MeshService mesh;
  const OrdersScreen({super.key, required this.mesh});

  @override
  State<OrdersScreen> createState() => _OrdersScreenState();
}

class _OrdersScreenState extends State<OrdersScreen> {
  List<Order> _orders = [];
  StreamSubscription<List<Order>>? _watchSub;

  @override
  void initState() {
    super.initState();
    _watchSub = widget.mesh.productDb.orders.watchAll().listen((orders) {
      setState(() => _orders = orders);
    });
  }

  @override
  void dispose() {
    _watchSub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: _orders.isEmpty
          ? const Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.receipt_long_outlined,
                      size: 48, color: Colors.grey),
                  SizedBox(height: 8),
                  Text('No orders yet'),
                  SizedBox(height: 4),
                  Text('Create an order from available products',
                      style: TextStyle(color: Colors.grey)),
                ],
              ),
            )
          : ListView.builder(
              itemCount: _orders.length,
              itemBuilder: (context, index) {
                final order = _orders[index];
                final product = widget.mesh.productDb.products
                    .findById(order.productId);
                final buyer =
                    widget.mesh.userDb.users.findById(order.buyerId);

                return Card(
                  margin: const EdgeInsets.symmetric(
                      horizontal: 12, vertical: 4),
                  child: ListTile(
                    leading: _statusIcon(order.status, theme),
                    title: Text(product?.name ?? 'Unknown Product'),
                    subtitle: Text(
                      'Buyer: ${buyer?.name ?? 'Unknown'} | '
                      'Status: ${order.status}',
                    ),
                    trailing: PopupMenuButton<String>(
                      onSelected: (status) => _updateStatus(order, status),
                      itemBuilder: (_) => [
                        const PopupMenuItem(
                            value: 'pending', child: Text('Pending')),
                        const PopupMenuItem(
                            value: 'confirmed', child: Text('Confirmed')),
                        const PopupMenuItem(
                            value: 'shipped', child: Text('Shipped')),
                        const PopupMenuItem(
                            value: 'delivered', child: Text('Delivered')),
                      ],
                    ),
                  ),
                );
              },
            ),
      floatingActionButton: FloatingActionButton(
        onPressed: _showCreateDialog,
        child: const Icon(Icons.add_shopping_cart),
      ),
    );
  }

  Widget _statusIcon(String status, ThemeData theme) {
    final (IconData icon, Color color) = switch (status) {
      'pending' => (Icons.hourglass_empty, Colors.orange),
      'confirmed' => (Icons.check_circle_outline, Colors.blue),
      'shipped' => (Icons.local_shipping, Colors.purple),
      'delivered' => (Icons.done_all, Colors.green),
      _ => (Icons.help_outline, Colors.grey),
    };
    return CircleAvatar(
      backgroundColor: color.withValues(alpha: 0.15),
      child: Icon(icon, color: color, size: 20),
    );
  }

  void _updateStatus(Order order, String status) {
    widget.mesh.productDb.orders.updateById(order.id, (o) {
      o.status = status;
      return o;
    });
  }

  void _showCreateDialog() {
    final products = widget.mesh.productDb.products.findAll();
    final users = widget.mesh.userDb.users.findAll();

    if (products.isEmpty || users.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Need at least one product and one user')),
      );
      return;
    }

    String? selectedProductId = products.first.id;
    String? selectedBuyerId = users.first.id;

    showDialog(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: const Text('New Order'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              DropdownButtonFormField<String>(
                initialValue: selectedProductId,
                decoration: const InputDecoration(labelText: 'Product'),
                items: products
                    .map((p) => DropdownMenuItem(
                        value: p.id,
                        child: Text('${p.name} (\$${p.price})')))
                    .toList(),
                onChanged: (v) =>
                    setDialogState(() => selectedProductId = v),
              ),
              const SizedBox(height: 8),
              DropdownButtonFormField<String>(
                initialValue: selectedBuyerId,
                decoration: const InputDecoration(labelText: 'Buyer'),
                items: users
                    .map((u) =>
                        DropdownMenuItem(value: u.id, child: Text(u.name)))
                    .toList(),
                onChanged: (v) =>
                    setDialogState(() => selectedBuyerId = v),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                if (selectedProductId != null && selectedBuyerId != null) {
                  widget.mesh.productDb.orders.create(Order(
                    productId: selectedProductId!,
                    buyerId: selectedBuyerId!,
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
