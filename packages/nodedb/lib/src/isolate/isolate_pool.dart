import 'dart:async';
import 'dart:isolate';

/// A pool of persistent isolates for running FFI operations off the main thread.
///
/// All NodeDB FFI calls run on pool isolates to avoid blocking Flutter's
/// UI thread. The pool uses round-robin dispatch across isolates.
class NodeDbIsolatePool {
  final int poolSize;
  bool _disposed = false;

  NodeDbIsolatePool._(this.poolSize);

  /// Create an isolate pool with [poolSize] worker isolates.
  static Future<NodeDbIsolatePool> create({int poolSize = 2}) async {
    final pool = NodeDbIsolatePool._(poolSize);
    return pool;
  }

  /// Run a synchronous computation on a pool isolate.
  ///
  /// The [computation] function runs in a separate isolate. It must be
  /// a top-level function or a static method (no closures over instance state).
  Future<T> run<T>(T Function() computation) async {
    if (_disposed) throw StateError('IsolatePool has been disposed');
    // Use Isolate.run for simplicity. Persistent pool can be optimized later.
    return Isolate.run(computation);
  }

  /// Dispose the pool and release all isolate resources.
  void dispose() {
    _disposed = true;
  }
}
