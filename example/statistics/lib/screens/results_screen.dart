import 'dart:math';

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../benchmark/benchmark_config.dart';
import '../benchmark/benchmark_result.dart';

class ResultsScreen extends StatelessWidget {
  final List<BenchmarkResult> results;
  final BenchmarkMode mode;
  final int throughputDurationSecs;

  const ResultsScreen({
    super.key,
    required this.results,
    required this.mode,
    this.throughputDurationSecs = 1,
  });

  @override
  Widget build(BuildContext context) {
    if (results.isEmpty) {
      return Scaffold(
        appBar: AppBar(title: const Text('Results')),
        body: const Center(child: Text('No results.')),
      );
    }

    if (mode == BenchmarkMode.throughput) {
      return _ThroughputResults(
        results: results,
        durationSecs: throughputDurationSecs,
      );
    }

    return DefaultTabController(
      length: BenchmarkOperation.values.length,
      child: Scaffold(
        appBar: AppBar(
          title: Text(
            'Results — ${BenchmarkConfig.formatCount(results.first.recordCount)} records',
          ),
          bottom: TabBar(
            isScrollable: true,
            tabs: BenchmarkOperation.values
                .map((op) => Tab(text: op.label))
                .toList(),
          ),
        ),
        body: TabBarView(
          children: BenchmarkOperation.values
              .map((op) => _LatencyOperationTab(results: results, operation: op))
              .toList(),
        ),
      ),
    );
  }
}

// ─── Shared ──────────────────────────────────────────────────────────

const _chartColors = [
  Color(0xFF2196F3), // NodeDB — blue
  Color(0xFF4CAF50), // sqflite — green
  Color(0xFFFF9800), // Hive — orange
  Color(0xFF9C27B0), // Drift — purple
  Color(0xFFE91E63), // ObjectBox — pink
  Color(0xFF009688), // Isar — teal
];

// ─── Latency mode ────────────────────────────────────────────────────

class _LatencyOperationTab extends StatelessWidget {
  final List<BenchmarkResult> results;
  final BenchmarkOperation operation;

  const _LatencyOperationTab({required this.results, required this.operation});

  @override
  Widget build(BuildContext context) {
    final stats = results
        .where((r) => r.operations.containsKey(operation.name))
        .map((r) => MapEntry(r.dbName, r.operations[operation.name]!))
        .where((e) => e.value.timingsMs.isNotEmpty)
        .toList();

    if (stats.isEmpty) {
      return const Center(child: Text('No data for this operation.'));
    }

    return SafeArea(
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          SizedBox(
            height: 280,
            child: _LatencyBarChart(stats: stats),
          ),
          const SizedBox(height: 24),
          _LatencyStatsTable(stats: stats),
        ],
      ),
    );
  }
}

class _LatencyBarChart extends StatelessWidget {
  final List<MapEntry<String, OperationStats>> stats;

  const _LatencyBarChart({required this.stats});

  @override
  Widget build(BuildContext context) {
    final maxMedian = stats.map((s) => s.value.median).reduce(max);
    final maxY = max(maxMedian * 1.3, 1.0);

    return BarChart(
      BarChartData(
        alignment: BarChartAlignment.spaceAround,
        maxY: maxY,
        barTouchData: BarTouchData(
          touchTooltipData: BarTouchTooltipData(
            getTooltipItem: (group, groupIndex, rod, rodIndex) {
              return BarTooltipItem(
                '${stats[groupIndex].key}\n${rod.toY.toStringAsFixed(1)} ms',
                const TextStyle(color: Colors.white, fontSize: 12),
              );
            },
          ),
        ),
        titlesData: FlTitlesData(
          show: true,
          bottomTitles: AxisTitles(
            sideTitles: SideTitles(
              showTitles: true,
              getTitlesWidget: (value, meta) {
                final idx = value.toInt();
                if (idx < 0 || idx >= stats.length) return const SizedBox();
                return Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    stats[idx].key,
                    style: const TextStyle(fontSize: 11),
                  ),
                );
              },
              reservedSize: 36,
            ),
          ),
          leftTitles: AxisTitles(
            axisNameWidget: const Text('ms', style: TextStyle(fontSize: 11)),
            sideTitles: SideTitles(
              showTitles: true,
              reservedSize: 48,
              getTitlesWidget: (value, meta) => Text(
                value.toStringAsFixed(0),
                style: const TextStyle(fontSize: 10),
              ),
            ),
          ),
          topTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
          rightTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
        ),
        gridData: FlGridData(
          show: true,
          drawVerticalLine: false,
          horizontalInterval: max(maxY / 5, 0.1),
        ),
        borderData: FlBorderData(show: false),
        barGroups: List.generate(stats.length, (i) {
          return BarChartGroupData(
            x: i,
            barRods: [
              BarChartRodData(
                toY: stats[i].value.median,
                color: _chartColors[i % _chartColors.length],
                width: 32,
                borderRadius: const BorderRadius.vertical(top: Radius.circular(4)),
              ),
            ],
          );
        }),
      ),
    );
  }
}

class _LatencyStatsTable extends StatelessWidget {
  final List<MapEntry<String, OperationStats>> stats;

  const _LatencyStatsTable({required this.stats});

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        headingTextStyle: Theme.of(context)
            .textTheme
            .bodySmall
            ?.copyWith(fontWeight: FontWeight.bold),
        dataTextStyle: Theme.of(context).textTheme.bodySmall,
        columnSpacing: 20,
        columns: const [
          DataColumn(label: Text('Database')),
          DataColumn(label: Text('Median'), numeric: true),
          DataColumn(label: Text('Mean'), numeric: true),
          DataColumn(label: Text('Min'), numeric: true),
          DataColumn(label: Text('Max'), numeric: true),
          DataColumn(label: Text('P95'), numeric: true),
          DataColumn(label: Text('P99'), numeric: true),
          DataColumn(label: Text('StdDev'), numeric: true),
        ],
        rows: stats.map((s) {
          final o = s.value;
          return DataRow(cells: [
            DataCell(Text(s.key)),
            DataCell(Text(_fmtMs(o.median))),
            DataCell(Text(_fmtMs(o.mean))),
            DataCell(Text(_fmtMs(o.minValue))),
            DataCell(Text(_fmtMs(o.maxValue))),
            DataCell(Text(_fmtMs(o.p95))),
            DataCell(Text(_fmtMs(o.p99))),
            DataCell(Text(_fmtMs(o.stdDev))),
          ]);
        }).toList(),
      ),
    );
  }

  String _fmtMs(double ms) {
    if (ms >= 1000) return '${(ms / 1000).toStringAsFixed(2)}s';
    return '${ms.toStringAsFixed(1)}ms';
  }
}

// ─── Throughput mode ─────────────────────────────────────────────────

class _ThroughputResults extends StatelessWidget {
  final List<BenchmarkResult> results;
  final int durationSecs;

  const _ThroughputResults({
    required this.results,
    required this.durationSecs,
  });

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: ThroughputOperation.values.length,
      child: Scaffold(
        appBar: AppBar(
          title: Text(
            'Throughput — ${durationSecs}s per operation',
          ),
          bottom: TabBar(
            isScrollable: true,
            tabs: ThroughputOperation.values
                .map((op) => Tab(text: op.label))
                .toList(),
          ),
        ),
        body: TabBarView(
          children: ThroughputOperation.values
              .map((op) =>
                  _ThroughputOperationTab(results: results, operation: op))
              .toList(),
        ),
      ),
    );
  }
}

class _ThroughputOperationTab extends StatelessWidget {
  final List<BenchmarkResult> results;
  final ThroughputOperation operation;

  const _ThroughputOperationTab({
    required this.results,
    required this.operation,
  });

  @override
  Widget build(BuildContext context) {
    final stats = results
        .where((r) => r.throughput.containsKey(operation.name))
        .map((r) => MapEntry(r.dbName, r.throughput[operation.name]!))
        .where((e) => e.value.opsCompleted > 0)
        .toList();

    if (stats.isEmpty) {
      return const Center(child: Text('No data for this operation.'));
    }

    return SafeArea(
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          SizedBox(
            height: 280,
            child: _ThroughputBarChart(stats: stats),
          ),
          const SizedBox(height: 24),
          _ThroughputStatsTable(stats: stats),
        ],
      ),
    );
  }
}

class _ThroughputBarChart extends StatelessWidget {
  final List<MapEntry<String, ThroughputStats>> stats;

  const _ThroughputBarChart({required this.stats});

  @override
  Widget build(BuildContext context) {
    final maxOps = stats.map((s) => s.value.opsPerSecond).reduce(max);
    final maxY = max(maxOps * 1.3, 1.0);

    return BarChart(
      BarChartData(
        alignment: BarChartAlignment.spaceAround,
        maxY: maxY,
        barTouchData: BarTouchData(
          touchTooltipData: BarTouchTooltipData(
            getTooltipItem: (group, groupIndex, rod, rodIndex) {
              return BarTooltipItem(
                '${stats[groupIndex].key}\n${_fmtOps(rod.toY)} ops/s',
                const TextStyle(color: Colors.white, fontSize: 12),
              );
            },
          ),
        ),
        titlesData: FlTitlesData(
          show: true,
          bottomTitles: AxisTitles(
            sideTitles: SideTitles(
              showTitles: true,
              getTitlesWidget: (value, meta) {
                final idx = value.toInt();
                if (idx < 0 || idx >= stats.length) return const SizedBox();
                return Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    stats[idx].key,
                    style: const TextStyle(fontSize: 11),
                  ),
                );
              },
              reservedSize: 36,
            ),
          ),
          leftTitles: AxisTitles(
            axisNameWidget:
                const Text('ops/s', style: TextStyle(fontSize: 11)),
            sideTitles: SideTitles(
              showTitles: true,
              reservedSize: 56,
              getTitlesWidget: (value, meta) => Text(
                _fmtOps(value),
                style: const TextStyle(fontSize: 10),
              ),
            ),
          ),
          topTitles:
              const AxisTitles(sideTitles: SideTitles(showTitles: false)),
          rightTitles:
              const AxisTitles(sideTitles: SideTitles(showTitles: false)),
        ),
        gridData: FlGridData(
          show: true,
          drawVerticalLine: false,
          horizontalInterval: max(maxY / 5, 0.1),
        ),
        borderData: FlBorderData(show: false),
        barGroups: List.generate(stats.length, (i) {
          return BarChartGroupData(
            x: i,
            barRods: [
              BarChartRodData(
                toY: stats[i].value.opsPerSecond,
                color: _chartColors[i % _chartColors.length],
                width: 32,
                borderRadius:
                    const BorderRadius.vertical(top: Radius.circular(4)),
              ),
            ],
          );
        }),
      ),
    );
  }

  String _fmtOps(double ops) {
    if (ops >= 1000000) return '${(ops / 1000000).toStringAsFixed(1)}M';
    if (ops >= 1000) return '${(ops / 1000).toStringAsFixed(1)}K';
    return ops.toStringAsFixed(0);
  }
}

class _ThroughputStatsTable extends StatelessWidget {
  final List<MapEntry<String, ThroughputStats>> stats;

  const _ThroughputStatsTable({required this.stats});

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        headingTextStyle: Theme.of(context)
            .textTheme
            .bodySmall
            ?.copyWith(fontWeight: FontWeight.bold),
        dataTextStyle: Theme.of(context).textTheme.bodySmall,
        columnSpacing: 20,
        columns: const [
          DataColumn(label: Text('Database')),
          DataColumn(label: Text('Ops Completed'), numeric: true),
          DataColumn(label: Text('Duration'), numeric: true),
          DataColumn(label: Text('Ops/sec'), numeric: true),
        ],
        rows: stats.map((s) {
          final t = s.value;
          return DataRow(cells: [
            DataCell(Text(s.key)),
            DataCell(Text(_fmtCount(t.opsCompleted))),
            DataCell(Text(_fmtMs(t.durationMs))),
            DataCell(Text(_fmtOps(t.opsPerSecond))),
          ]);
        }).toList(),
      ),
    );
  }

  String _fmtMs(double ms) {
    if (ms >= 1000) return '${(ms / 1000).toStringAsFixed(2)}s';
    return '${ms.toStringAsFixed(1)}ms';
  }

  String _fmtOps(double ops) {
    if (ops >= 1000000) return '${(ops / 1000000).toStringAsFixed(1)}M';
    if (ops >= 1000) return '${(ops / 1000).toStringAsFixed(1)}K';
    return ops.toStringAsFixed(0);
  }

  String _fmtCount(int count) {
    if (count >= 1000000) return '${(count / 1000000).toStringAsFixed(1)}M';
    if (count >= 1000) return '${(count / 1000).toStringAsFixed(1)}K';
    return '$count';
  }
}
