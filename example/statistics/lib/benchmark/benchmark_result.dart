import 'dart:math';

class OperationStats {
  final String operation;
  final List<double> timingsMs;

  OperationStats({required this.operation, required this.timingsMs});

  double get mean =>
      timingsMs.isEmpty ? 0 : timingsMs.reduce((a, b) => a + b) / timingsMs.length;

  double get median {
    if (timingsMs.isEmpty) return 0;
    final sorted = List<double>.from(timingsMs)..sort();
    final mid = sorted.length ~/ 2;
    return sorted.length.isOdd
        ? sorted[mid]
        : (sorted[mid - 1] + sorted[mid]) / 2;
  }

  double get minValue => timingsMs.isEmpty ? 0 : timingsMs.reduce(min);

  double get maxValue => timingsMs.isEmpty ? 0 : timingsMs.reduce(max);

  double get stdDev {
    if (timingsMs.length < 2) return 0;
    final m = mean;
    final variance =
        timingsMs.map((t) => (t - m) * (t - m)).reduce((a, b) => a + b) /
            (timingsMs.length - 1);
    return sqrt(variance);
  }

  double get p95 => _percentile(0.95);
  double get p99 => _percentile(0.99);

  double _percentile(double p) {
    if (timingsMs.isEmpty) return 0;
    final sorted = List<double>.from(timingsMs)..sort();
    final index = (p * (sorted.length - 1)).round();
    return sorted[index];
  }
}

class ThroughputStats {
  final String operation;
  final int opsCompleted;
  final double durationMs;

  ThroughputStats({
    required this.operation,
    required this.opsCompleted,
    required this.durationMs,
  });

  double get opsPerSecond =>
      durationMs > 0 ? opsCompleted / (durationMs / 1000.0) : 0;
}

class BenchmarkResult {
  final String dbName;
  final int recordCount;
  final Map<String, OperationStats> operations;
  final Map<String, ThroughputStats> throughput;

  BenchmarkResult({
    required this.dbName,
    required this.recordCount,
    required this.operations,
    this.throughput = const {},
  });
}

enum BenchmarkOperation {
  bulkInsert('Bulk Insert'),
  readById('Read by ID'),
  readAll('Read All'),
  filteredQuery('Filtered Query'),
  search('Search'),
  bulkUpdate('Bulk Update'),
  bulkDelete('Bulk Delete');

  final String label;
  const BenchmarkOperation(this.label);
}

enum ThroughputOperation {
  singleInsert('Single Insert'),
  singleRead('Single Read'),
  singleUpdate('Single Update'),
  singleDelete('Single Delete'),
  queryByAge('Query by Age'),
  searchByName('Search');

  final String label;
  const ThroughputOperation(this.label);
}
