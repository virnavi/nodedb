import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';
import 'package:nodedb_inspector_flutter/nodedb_inspector_flutter.dart';
import 'package:nodedb_example/database/mesh_service.dart';
import 'package:nodedb_example/screens/users_screen.dart';
import 'package:nodedb_example/screens/products_screen.dart';
import 'package:nodedb_example/screens/orders_screen.dart';
import 'package:nodedb_example/screens/mesh_screen.dart';
import 'package:nodedb_example/screens/cache_screen.dart';

class NodeDbExampleApp extends StatelessWidget {
  final MeshService mesh;
  const NodeDbExampleApp({super.key, required this.mesh});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'NodeDB Mesh Demo',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: _HomeScreen(mesh: mesh),
    );
  }
}

class _HomeScreen extends StatefulWidget {
  final MeshService mesh;
  const _HomeScreen({required this.mesh});

  @override
  State<_HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<_HomeScreen> {
  int _selectedIndex = 0;

  late final NodeDbInspector _inspector;

  @override
  void initState() {
    super.initState();
    _inspector = NodeDbInspector.mesh([
      widget.mesh.userDb.db,
      widget.mesh.productDb.db,
    ]);
  }

  @override
  Widget build(BuildContext context) {
    final screens = [
      UsersScreen(mesh: widget.mesh),
      ProductsScreen(mesh: widget.mesh),
      OrdersScreen(mesh: widget.mesh),
      CacheScreen(mesh: widget.mesh),
      MeshScreen(mesh: widget.mesh),
      InspectorScreen(inspector: _inspector),
    ];

    return Scaffold(
      appBar: AppBar(
        title: const Text('NodeDB Mesh Demo'),
        actions: [
          IconButton(
            icon: const Icon(Icons.info_outline),
            onPressed: () => _showAbout(context),
          ),
        ],
      ),
      body: SafeArea(
        child: screens[_selectedIndex],
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (i) => setState(() => _selectedIndex = i),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.people), label: 'Users'),
          NavigationDestination(
              icon: Icon(Icons.inventory), label: 'Products'),
          NavigationDestination(
              icon: Icon(Icons.receipt_long), label: 'Orders'),
          NavigationDestination(icon: Icon(Icons.timer), label: 'Cache'),
          NavigationDestination(icon: Icon(Icons.hub), label: 'Mesh'),
          NavigationDestination(
              icon: Icon(Icons.bug_report), label: 'Inspect'),
        ],
      ),
    );
  }

  void _showAbout(BuildContext context) {
    final userCount = widget.mesh.userDb.users.count();
    final productCount = widget.mesh.productDb.products.count();
    final orderCount = widget.mesh.productDb.orders.count();

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('NodeDB Mesh Demo'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Multi-domain databases in a single mesh.\n'
              'Install on 2 devices to see cross-device federation.\n',
            ),
            Text('Users DB: $userCount users'),
            Text('Products DB: $productCount products, $orderCount orders'),
            const Text('\nMesh: nodedb-example'),
            const Text('Databases: users, products'),
            const Text('Sharing: full'),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}
