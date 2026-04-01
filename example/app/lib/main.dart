import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:nodedb_example/app.dart';
import 'package:nodedb_example/database/mesh_service.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _AppLoader());
}

class _AppLoader extends StatefulWidget {
  const _AppLoader();

  @override
  State<_AppLoader> createState() => _AppLoaderState();
}

class _AppLoaderState extends State<_AppLoader> {
  MeshService? _mesh;
  String? _error;

  @override
  void initState() {
    super.initState();
    _initDatabase();
  }

  Future<void> _initDatabase() async {
    try {
      final appDir = await getApplicationDocumentsDirectory();
      final dbDir = '${appDir.path}${Platform.pathSeparator}nodedb_example';

      final mesh = MeshService();
      mesh.init(dbDir);

      if (mounted) setState(() => _mesh = mesh);
    } catch (e, st) {
      debugPrint('NodeDB init failed: $e\n$st');
      if (mounted) setState(() => _error = e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return MaterialApp(
        home: Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Text('Failed to initialize database:\n$_error',
                  style: const TextStyle(color: Colors.red)),
            ),
          ),
        ),
      );
    }

    if (_mesh == null) {
      return const MaterialApp(
        home: Scaffold(
          body: Center(child: CircularProgressIndicator()),
        ),
      );
    }

    return NodeDbExampleApp(mesh: _mesh!);
  }
}
