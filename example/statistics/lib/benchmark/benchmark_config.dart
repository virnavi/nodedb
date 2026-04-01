enum BenchmarkMode {
  latency('Latency (ms)'),
  throughput('Throughput (ops/sec)');

  final String label;
  const BenchmarkMode(this.label);
}

class BenchmarkConfig {
  final int recordCount;
  final int warmupRuns;
  final int measurementRuns;
  final BenchmarkMode mode;
  final int throughputDurationSecs;

  const BenchmarkConfig({
    required this.recordCount,
    this.warmupRuns = 2,
    this.measurementRuns = 5,
    this.mode = BenchmarkMode.latency,
    this.throughputDurationSecs = 1,
  });

  static const presets = [1000, 10000, 100000, 1000000];

  static String formatCount(int count) {
    if (count >= 1000000) return '${count ~/ 1000000}M';
    if (count >= 1000) return '${count ~/ 1000}K';
    return '$count';
  }
}
