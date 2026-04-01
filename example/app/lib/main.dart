import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:nodedb_example/app.dart';
import 'package:nodedb_example/database/mesh_service.dart';

/// Top-level variable survives hot restart (Dart VM keeps it alive).
MeshService? _meshService;

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _SplashLoader());
}

class _SplashLoader extends StatefulWidget {
  const _SplashLoader();

  @override
  State<_SplashLoader> createState() => _SplashLoaderState();
}

class _SplashLoaderState extends State<_SplashLoader> {
  String? _error;
  bool _ready = false;

  @override
  void initState() {
    super.initState();
    _initDatabase();
  }

  Future<void> _initDatabase() async {
    // Already initialized (hot restart) — skip straight to app.
    if (_meshService != null) {
      if (mounted) setState(() => _ready = true);
      return;
    }

    try {
      final appDir = await getApplicationDocumentsDirectory();
      final dbDir = '${appDir.path}${Platform.pathSeparator}nodedb_example';

      // Retry a few times — a previous instance may still be releasing locks.
      const maxRetries = 3;
      for (var i = 0; i < maxRetries; i++) {
        try {
          _meshService = MeshService()..init(dbDir);
          break;
        } catch (e) {
          if (i < maxRetries - 1) {
            await Future.delayed(const Duration(seconds: 1));
          } else {
            rethrow;
          }
        }
      }

      if (mounted) setState(() => _ready = true);
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

    if (!_ready) {
      return MaterialApp(
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
          useMaterial3: true,
        ),
        home: const Scaffold(
          body: Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(Icons.storage, size: 64, color: Colors.indigo),
                SizedBox(height: 24),
                Text('NodeDB', style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold)),
                SizedBox(height: 8),
                Text('Initializing database...', style: TextStyle(color: Colors.grey)),
                SizedBox(height: 24),
                CircularProgressIndicator(),
              ],
            ),
          ),
        ),
      );
    }

    return NodeDbExampleApp(mesh: _meshService!);
  }
}
