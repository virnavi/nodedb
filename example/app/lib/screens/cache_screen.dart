import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart' hide SearchResult;
import 'package:nodedb_example/database/mesh_service.dart';
import 'package:nodedb_example/models/product_models.dart';

/// Cache demo screen — demonstrates per-record TTL cache configuration.
class CacheScreen extends StatefulWidget {
  final MeshService mesh;
  const CacheScreen({super.key, required this.mesh});

  @override
  State<CacheScreen> createState() => _CacheScreenState();
}

class _CacheScreenState extends State<CacheScreen> {
  List<SearchResult> _results = [];
  String _status = '';

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  void _refresh() {
    setState(() {
      _results = widget.mesh.productDb.searchResults.findAll();
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Column(
        children: [
          if (_status.isNotEmpty)
            Padding(
              padding: const EdgeInsets.all(12),
              child: Text(_status, style: const TextStyle(color: Colors.blue)),
            ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12),
            child: Wrap(
              spacing: 8,
              children: [
                ActionChip(
                  label: const Text('Add Cached (60s)'),
                  onPressed: _addCached60s,
                ),
                ActionChip(
                  label: const Text('Add Cached (5s)'),
                  onPressed: _addCached5s,
                ),
                ActionChip(
                  label: const Text('Sweep Expired'),
                  onPressed: _sweepExpired,
                ),
                ActionChip(
                  label: const Text('Clear All'),
                  onPressed: _clearAll,
                ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: _results.isEmpty
                ? const Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(Icons.timer_outlined,
                            size: 48, color: Colors.grey),
                        SizedBox(height: 8),
                        Text('No cached search results'),
                        SizedBox(height: 4),
                        Text('Add cached entries to see TTL in action',
                            style: TextStyle(color: Colors.grey)),
                      ],
                    ),
                  )
                : ListView.builder(
                    itemCount: _results.length,
                    itemBuilder: (context, index) {
                      final result = _results[index];
                      final cache = widget.mesh.productDb.searchResults
                          .findById(result.id);
                      return Card(
                        margin: const EdgeInsets.symmetric(
                            horizontal: 12, vertical: 4),
                        child: ListTile(
                          leading: const CircleAvatar(
                            child: Icon(Icons.search),
                          ),
                          title: Text('Query: "${result.query}"'),
                          subtitle: Text(
                            'Cached at: ${result.cachedAt?.toLocal() ?? "unknown"}\n'
                            'Result: ${result.resultJson}',
                          ),
                          isThreeLine: true,
                          trailing: cache != null
                              ? const Icon(Icons.check_circle,
                                  color: Colors.green)
                              : const Icon(Icons.cancel, color: Colors.red),
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  void _addCached60s() {
    widget.mesh.productDb.searchResults.createWithCache(
      SearchResult(
        query: 'flutter database ${DateTime.now().second}',
        resultJson: '{"count": 42, "source": "cache_demo"}',
        cachedAt: DateTime.now(),
      ),
      const CacheConfig(
        mode: CacheMode.expireAfterWrite,
        ttl: Duration(seconds: 60),
      ),
    );
    setState(() => _status = 'Added cached result (TTL: 60s)');
    _refresh();
  }

  void _addCached5s() {
    widget.mesh.productDb.searchResults.createWithCache(
      SearchResult(
        query: 'short-lived ${DateTime.now().second}',
        resultJson: '{"count": 7, "source": "quick_cache"}',
        cachedAt: DateTime.now(),
      ),
      const CacheConfig(
        mode: CacheMode.expireAfterWrite,
        ttl: Duration(seconds: 5),
      ),
    );
    setState(() => _status = 'Added cached result (TTL: 5s)');
    _refresh();
  }

  void _sweepExpired() {
    final deleted = widget.mesh.productDb.searchResults.sweepExpired();
    setState(() => _status = 'Swept $deleted expired record(s)');
    _refresh();
  }

  void _clearAll() {
    final results = widget.mesh.productDb.searchResults.findAll();
    if (results.isNotEmpty) {
      widget.mesh.productDb.searchResults
          .deleteAllById(results.map((r) => r.id).toList());
    }
    setState(() => _status = 'Cleared all cached results');
    _refresh();
  }
}
