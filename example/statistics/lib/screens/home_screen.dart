import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import '../adapters/db_adapter.dart';
import '../adapters/nodedb_adapter.dart';
import '../adapters/sqflite_adapter.dart';
import '../adapters/hive_adapter.dart';
import '../adapters/drift_adapter.dart';
import '../adapters/objectbox_adapter.dart';
import '../adapters/isar_adapter.dart';
import '../benchmark/benchmark_config.dart';
import 'running_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final _adapters = <DbAdapter, bool>{
    NodeDbAdapter(): true,
    SqfliteAdapter(): true,
    HiveAdapter(): true,
    DriftAdapter(): true,
    ObjectBoxAdapter(): true,
    IsarAdapter(): true,
  };

  int _recordCount = 1000;
  int _measurementRuns = 5;
  BenchmarkMode _mode = BenchmarkMode.latency;
  int _throughputDurationSecs = 1;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('NodeDB Benchmark'),
        centerTitle: true,
      ),
      body: SafeArea(
        child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Databases',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  ..._adapters.entries.map((e) => CheckboxListTile(
                        title: Text(e.key.name),
                        value: e.value,
                        dense: true,
                        onChanged: (v) =>
                            setState(() => _adapters[e.key] = v ?? false),
                      )),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Benchmark Mode',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  SegmentedButton<BenchmarkMode>(
                    segments: BenchmarkMode.values
                        .map((m) => ButtonSegment(
                              value: m,
                              label: Text(m.label),
                            ))
                        .toList(),
                    selected: {_mode},
                    onSelectionChanged: (v) =>
                        setState(() => _mode = v.first),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    _mode == BenchmarkMode.latency
                        ? 'Measures time to complete each bulk operation.'
                        : 'Counts how many individual operations complete in the given time.',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          ),
          if (_mode == BenchmarkMode.latency) ...[
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Record Count',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 8,
                      children: BenchmarkConfig.presets.map((count) {
                        return ChoiceChip(
                          label: Text(BenchmarkConfig.formatCount(count)),
                          selected: _recordCount == count,
                          onSelected: (_) =>
                              setState(() => _recordCount = count),
                        );
                      }).toList(),
                    ),
                  ],
                ),
              ),
            ),
          ],
          if (_mode == BenchmarkMode.throughput) ...[
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Duration (seconds)',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    Wrap(
                      spacing: 8,
                      children: [1, 3, 5, 10].map((secs) {
                        return ChoiceChip(
                          label: Text('${secs}s'),
                          selected: _throughputDurationSecs == secs,
                          onSelected: (_) =>
                              setState(() => _throughputDurationSecs = secs),
                        );
                      }).toList(),
                    ),
                  ],
                ),
              ),
            ),
          ],
          if (_mode == BenchmarkMode.latency) ...[
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Measurement Runs',
                        style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    Slider(
                      value: _measurementRuns.toDouble(),
                      min: 1,
                      max: 10,
                      divisions: 9,
                      label: '$_measurementRuns',
                      onChanged: (v) =>
                          setState(() => _measurementRuns = v.round()),
                    ),
                  ],
                ),
              ),
            ),
          ],
          const SizedBox(height: 24),
          FilledButton.icon(
            onPressed: _selectedAdapters.isEmpty ? null : _startBenchmark,
            icon: const Icon(Icons.play_arrow),
            label: const Text('Start Benchmark'),
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 48),
            ),
          ),
          if (_mode == BenchmarkMode.latency && _recordCount >= 100000) ...[
            const SizedBox(height: 8),
            Text(
              'Large record counts may take several minutes.',
              style: Theme.of(context).textTheme.bodySmall,
              textAlign: TextAlign.center,
            ),
          ],
        ],
      ),
      ),
    );
  }

  List<DbAdapter> get _selectedAdapters =>
      _adapters.entries.where((e) => e.value).map((e) => e.key).toList();

  Future<void> _startBenchmark() async {
    final dir = await getApplicationDocumentsDirectory();
    if (!mounted) return;

    Navigator.of(context).push(MaterialPageRoute(
      builder: (_) => RunningScreen(
        adapters: _selectedAdapters,
        config: BenchmarkConfig(
          recordCount: _recordCount,
          measurementRuns: _measurementRuns,
          mode: _mode,
          throughputDurationSecs: _throughputDurationSecs,
        ),
        basePath: dir.path,
      ),
    ));
  }
}
