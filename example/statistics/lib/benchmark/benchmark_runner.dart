import 'dart:isolate';
import 'dart:math';

import 'package:flutter/services.dart';

import '../adapters/db_adapter.dart';
import '../adapters/nodedb_adapter.dart';
import '../adapters/sqflite_adapter.dart';
import '../adapters/hive_adapter.dart';
import '../adapters/drift_adapter.dart';
import '../adapters/objectbox_adapter.dart';
import '../adapters/isar_adapter.dart';
import '../models/test_record.dart';
import 'benchmark_config.dart';
import 'benchmark_result.dart';

typedef ProgressCallback = void Function(
    String dbName, String message, double progress);

/// Creates a [DbAdapter] by name (used inside isolates).
DbAdapter _createAdapter(String name) {
  switch (name) {
    case 'NodeDB':
      return NodeDbAdapter();
    case 'sqflite':
      return SqfliteAdapter();
    case 'Hive CE':
      return HiveAdapter();
    case 'Drift':
      return DriftAdapter();
    case 'ObjectBox':
      return ObjectBoxAdapter();
    case 'Isar':
      return IsarAdapter();
    default:
      throw ArgumentError('Unknown adapter: $name');
  }
}

/// Top-level isolate entry point. Initialises the platform channel bridge
/// so that plugins like sqflite and ObjectBox work in background isolates,
/// then runs the full benchmark for one database adapter and returns a
/// plain-Map result that is guaranteed sendable.
Future<Map<String, dynamic>> _isolateEntry(Map<String, dynamic> params) async {
  // Enable platform channels in this background isolate
  BackgroundIsolateBinaryMessenger.ensureInitialized(
    params['token'] as RootIsolateToken,
  );

  final adapterName = params['adapterName'] as String;
  final basePath = params['basePath'] as String;
  final recordCount = params['recordCount'] as int;
  final warmupRuns = params['warmupRuns'] as int;
  final measurementRuns = params['measurementRuns'] as int;
  final mode = params['mode'] as String;
  final throughputDurationSecs = params['throughputDurationSecs'] as int;

  final adapter = _createAdapter(adapterName);
  final operations = <String, Map<String, dynamic>>{};
  final throughput = <String, Map<String, dynamic>>{};
  String? error;

  try {
    await adapter.open(basePath);
    await adapter.clear();

    if (mode == 'latency') {
      for (final op in BenchmarkOperation.values) {
        final timings = await _benchmarkOp(
          adapter, op, recordCount, warmupRuns, measurementRuns,
        );
        operations[op.name] = {
          'label': op.label,
          'timings': timings,
        };
      }
    } else {
      for (final op in ThroughputOperation.values) {
        final result = await _benchmarkThroughput(
          adapter, op, recordCount, throughputDurationSecs,
        );
        throughput[op.name] = result;
      }
    }
  } catch (e, st) {
    error = '$e\n$st';
  } finally {
    try {
      await adapter.close();
    } catch (_) {}
  }

  return {
    'dbName': adapterName,
    'recordCount': recordCount,
    'operations': operations,
    'throughput': throughput,
    'error': error,
  };
}

Future<List<double>> _benchmarkOp(
  DbAdapter adapter,
  BenchmarkOperation op,
  int recordCount,
  int warmupRuns,
  int measurementRuns,
) async {
  final timings = <double>[];

  for (var i = 0; i < warmupRuns; i++) {
    await _runOp(adapter, op, recordCount);
    await adapter.clear();
  }

  for (var i = 0; i < measurementRuns; i++) {
    final sw = Stopwatch()..start();
    await _runOp(adapter, op, recordCount);
    sw.stop();
    timings.add(sw.elapsedMicroseconds / 1000.0);
    await adapter.clear();
  }

  await adapter.clear();
  return timings;
}

Future<void> _runOp(DbAdapter adapter, BenchmarkOperation op, int count) async {
  final records = TestRecord.generate(count);
  final rng = Random(42);

  switch (op) {
    case BenchmarkOperation.bulkInsert:
      await adapter.insertBatch(records);

    case BenchmarkOperation.readById:
      await adapter.insertBatch(records);
      final sampleSize = min(1000, count);
      for (var i = 0; i < sampleSize; i++) {
        await adapter.getById(rng.nextInt(count) + 1);
      }

    case BenchmarkOperation.readAll:
      await adapter.insertBatch(records);
      await adapter.getAll();

    case BenchmarkOperation.filteredQuery:
      await adapter.insertBatch(records);
      await adapter.queryByAge(50);

    case BenchmarkOperation.search:
      await adapter.insertBatch(records);
      // Search for a few different terms
      await adapter.searchByName('User 1');
      await adapter.searchByName('User 50');
      await adapter.searchByName('User 999');

    case BenchmarkOperation.bulkUpdate:
      await adapter.insertBatch(records);
      await adapter.updateBatch(
        records.map((r) => r.copyWith(score: r.score + 1)).toList(),
      );

    case BenchmarkOperation.bulkDelete:
      await adapter.insertBatch(records);
      await adapter.deleteBatch(List.generate(count, (i) => i + 1));
  }
}

/// Runs a single throughput operation for the given duration, counting
/// how many iterations complete within the time limit.
Future<Map<String, dynamic>> _benchmarkThroughput(
  DbAdapter adapter,
  ThroughputOperation op,
  int recordCount,
  int durationSecs,
) async {
  final records = TestRecord.generate(recordCount);
  final rng = Random(42);
  final deadline = Duration(seconds: durationSecs);

  // Seed data for read/query/update operations
  if (op != ThroughputOperation.singleInsert) {
    await adapter.clear();
    await adapter.insertBatch(records);
  }

  int opsCompleted = 0;
  final sw = Stopwatch()..start();

  while (sw.elapsed < deadline) {
    switch (op) {
      case ThroughputOperation.singleInsert:
        final id = recordCount + opsCompleted + 1;
        await adapter.insertBatch([
          TestRecord(
            id: id,
            name: 'Bench $id',
            email: 'bench$id@test.com',
            age: 25,
            score: 5.0,
            createdAt: DateTime.now(),
          ),
        ]);

      case ThroughputOperation.singleRead:
        await adapter.getById(rng.nextInt(recordCount) + 1);

      case ThroughputOperation.singleUpdate:
        final idx = rng.nextInt(recordCount);
        final rec = records[idx].copyWith(score: records[idx].score + 1);
        await adapter.updateBatch([rec]);

      case ThroughputOperation.singleDelete:
        // Delete then re-insert to keep data available
        final id = rng.nextInt(recordCount) + 1;
        await adapter.deleteBatch([id]);
        await adapter.insertBatch([records[id - 1]]);

      case ThroughputOperation.queryByAge:
        await adapter.queryByAge(50);

      case ThroughputOperation.searchByName:
        await adapter.searchByName('User ${rng.nextInt(recordCount) + 1}');
    }
    opsCompleted++;
  }

  sw.stop();
  await adapter.clear();

  return {
    'label': op.label,
    'opsCompleted': opsCompleted,
    'durationMs': sw.elapsedMicroseconds / 1000.0,
  };
}

/// Runs benchmark directly in the calling isolate (no Isolate.run).
/// Used as fallback when background isolate execution fails.
Future<Map<String, dynamic>> _runDirect({
  required String adapterName,
  required String basePath,
  required int recordCount,
  required int warmupRuns,
  required int measurementRuns,
  required String mode,
  required int throughputDurationSecs,
}) async {
  final adapter = _createAdapter(adapterName);
  final operations = <String, Map<String, dynamic>>{};
  final throughput = <String, Map<String, dynamic>>{};
  String? error;

  try {
    await adapter.open(basePath);
    await adapter.clear();

    if (mode == 'latency') {
      for (final op in BenchmarkOperation.values) {
        final timings = await _benchmarkOp(
          adapter, op, recordCount, warmupRuns, measurementRuns,
        );
        operations[op.name] = {
          'label': op.label,
          'timings': timings,
        };
      }
    } else {
      for (final op in ThroughputOperation.values) {
        final result = await _benchmarkThroughput(
          adapter, op, recordCount, throughputDurationSecs,
        );
        throughput[op.name] = result;
      }
    }
  } catch (e, st) {
    error = '$e\n$st';
  } finally {
    try {
      await adapter.close();
    } catch (_) {}
  }

  return {
    'dbName': adapterName,
    'recordCount': recordCount,
    'operations': operations,
    'throughput': throughput,
    'error': error,
  };
}

class BenchmarkRunner {
  final BenchmarkConfig config;
  final List<DbAdapter> adapters;
  final ProgressCallback? onProgress;

  BenchmarkRunner({
    required this.config,
    required this.adapters,
    this.onProgress,
  });

  /// Runs each adapter's benchmark in a separate background isolate,
  /// one database at a time.
  Future<List<BenchmarkResult>> runAll(String basePath) async {
    final results = <BenchmarkResult>[];
    final token = RootIsolateToken.instance!;
    final mode = config.mode == BenchmarkMode.latency ? 'latency' : 'throughput';

    for (var i = 0; i < adapters.length; i++) {
      final adapter = adapters[i];
      _report(adapter.name, 'Running in isolate...', i / adapters.length);

      try {
        // Capture all params as a plain Map so everything is sendable
        final adapterName = adapter.name;
        final recordCount = config.recordCount;
        final warmupRuns = config.warmupRuns;
        final measurementRuns = config.measurementRuns;
        final throughputDurationSecs = config.throughputDurationSecs;

        var raw = await Isolate.run<Map<String, dynamic>>(() {
          return _isolateEntry({
            'token': token,
            'adapterName': adapterName,
            'basePath': basePath,
            'recordCount': recordCount,
            'warmupRuns': warmupRuns,
            'measurementRuns': measurementRuns,
            'mode': mode,
            'throughputDurationSecs': throughputDurationSecs,
          });
        });

        // If isolate run produced an error, retry directly in main isolate
        if (raw['error'] != null) {
          _report(adapter.name, 'Isolate failed, retrying directly...',
              i / adapters.length);
          raw = await _runDirect(
            adapterName: adapterName,
            basePath: basePath,
            recordCount: recordCount,
            warmupRuns: warmupRuns,
            measurementRuns: measurementRuns,
            mode: mode,
            throughputDurationSecs: throughputDurationSecs,
          );
        }

        results.add(_decodeResult(raw));

        final err = raw['error'] as String?;
        if (err != null) {
          _report(adapter.name, 'Error: $err', (i + 1) / adapters.length);
        } else {
          _report(adapter.name, 'Complete', (i + 1) / adapters.length);
        }
      } catch (e) {
        // Isolate failed entirely — try running directly
        _report(adapter.name, 'Isolate error, retrying directly...',
            i / adapters.length);
        try {
          final raw = await _runDirect(
            adapterName: adapter.name,
            basePath: basePath,
            recordCount: config.recordCount,
            warmupRuns: config.warmupRuns,
            measurementRuns: config.measurementRuns,
            mode: mode,
            throughputDurationSecs: config.throughputDurationSecs,
          );
          results.add(_decodeResult(raw));
          final err = raw['error'] as String?;
          if (err != null) {
            _report(adapter.name, 'Error: $err', (i + 1) / adapters.length);
          } else {
            _report(adapter.name, 'Complete', (i + 1) / adapters.length);
          }
        } catch (e2) {
          _report(adapter.name, 'Failed: $e2', (i + 1) / adapters.length);
          results.add(BenchmarkResult(
            dbName: adapter.name,
            recordCount: config.recordCount,
            operations: {},
          ));
        }
      }
    }

    _report('', 'Done', 1.0);
    return results;
  }

  static BenchmarkResult _decodeResult(Map<String, dynamic> raw) {
    final opsRaw = raw['operations'] as Map<String, dynamic>;
    final ops = <String, OperationStats>{};
    for (final entry in opsRaw.entries) {
      final opData = entry.value as Map<String, dynamic>;
      ops[entry.key] = OperationStats(
        operation: opData['label'] as String,
        timingsMs: (opData['timings'] as List).cast<double>(),
      );
    }

    final tpRaw = raw['throughput'] as Map<String, dynamic>? ?? {};
    final tp = <String, ThroughputStats>{};
    for (final entry in tpRaw.entries) {
      final data = entry.value as Map<String, dynamic>;
      tp[entry.key] = ThroughputStats(
        operation: data['label'] as String,
        opsCompleted: data['opsCompleted'] as int,
        durationMs: (data['durationMs'] as num).toDouble(),
      );
    }

    return BenchmarkResult(
      dbName: raw['dbName'] as String,
      recordCount: raw['recordCount'] as int,
      operations: ops,
      throughput: tp,
    );
  }

  void _report(String dbName, String message, double progress) {
    onProgress?.call(dbName, message, progress);
  }
}
