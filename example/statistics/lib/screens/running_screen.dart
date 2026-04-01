import 'package:flutter/material.dart';

import '../adapters/db_adapter.dart';
import '../benchmark/benchmark_config.dart';
import '../benchmark/benchmark_runner.dart';
import 'results_screen.dart';

class RunningScreen extends StatefulWidget {
  final List<DbAdapter> adapters;
  final BenchmarkConfig config;
  final String basePath;

  const RunningScreen({
    super.key,
    required this.adapters,
    required this.config,
    required this.basePath,
  });

  @override
  State<RunningScreen> createState() => _RunningScreenState();
}

class _RunningScreenState extends State<RunningScreen> {
  double _progress = 0;
  String _currentDb = '';
  String _currentOp = '';
  final _log = <String>[];
  bool _running = true;

  @override
  void initState() {
    super.initState();
    _run();
  }

  Future<void> _run() async {
    final runner = BenchmarkRunner(
      config: widget.config,
      adapters: widget.adapters,
      onProgress: (db, msg, progress) {
        if (!mounted) return;
        setState(() {
          _currentDb = db;
          _currentOp = msg;
          _progress = progress;
          if (db.isNotEmpty) {
            _log.add('[$db] $msg');
          }
        });
      },
    );

    final results = await runner.runAll(widget.basePath);

    if (!mounted) return;
    setState(() => _running = false);

    Navigator.of(context).pushReplacement(MaterialPageRoute(
      builder: (_) => ResultsScreen(
        results: results,
        mode: widget.config.mode,
        throughputDurationSecs: widget.config.throughputDurationSecs,
      ),
    ));
  }

  @override
  Widget build(BuildContext context) {
    return PopScope(
      canPop: !_running,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Running Benchmark'),
          automaticallyImplyLeading: !_running,
        ),
        body: SafeArea(
          child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              LinearProgressIndicator(value: _progress),
              const SizedBox(height: 16),
              if (_currentDb.isNotEmpty)
                Text(
                  '$_currentDb — $_currentOp',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              const SizedBox(height: 8),
              Text(
                '${(_progress * 100).toStringAsFixed(0)}% complete  |  '
                '${widget.config.mode == BenchmarkMode.latency ? '${BenchmarkConfig.formatCount(widget.config.recordCount)} records  |  ${widget.config.measurementRuns} runs' : '${widget.config.throughputDurationSecs}s per operation'}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const SizedBox(height: 16),
              Expanded(
                child: Container(
                  decoration: BoxDecoration(
                    color: Theme.of(context).colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: ListView.builder(
                    padding: const EdgeInsets.all(12),
                    reverse: true,
                    itemCount: _log.length,
                    itemBuilder: (_, i) => Padding(
                      padding: const EdgeInsets.only(bottom: 4),
                      child: Text(
                        _log[_log.length - 1 - i],
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        ),
      ),
    );
  }
}
