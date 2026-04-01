import 'dart:async';

import '../engine/nosql_engine.dart';

/// Type of sync event.
enum SyncEventType {
  /// A write happened locally (via DAO or direct engine call).
  localChange,

  /// A change was detected via the sync version counter (may be remote-applied).
  remoteChange,
}

/// A notification that data has changed.
class SyncEvent {
  /// The collection name, or `'*'` for a global version bump.
  final String collection;

  /// Whether this was a local or remote change.
  final SyncEventType type;

  const SyncEvent({required this.collection, required this.type});
}

/// Broadcasts [SyncEvent]s by polling the Rust sync version counter.
///
/// DAOs call [notifyLocal] after writes; the poll loop detects remote changes.
class CollectionNotifier {
  final NoSqlEngine _engine;
  Timer? _pollTimer;
  int _lastVersion = 0;
  final _controller = StreamController<SyncEvent>.broadcast();

  /// All change events (local + remote).
  Stream<SyncEvent> get changes => _controller.stream;

  /// Events filtered to a specific collection (or global `'*'` events).
  Stream<SyncEvent> collectionChanges(String name) =>
      changes.where((e) => e.collection == '*' || e.collection == name);

  CollectionNotifier(this._engine);

  /// Start polling the sync version counter.
  void startPolling({Duration interval = const Duration(milliseconds: 100)}) {
    _lastVersion = _engine.syncVersion();
    _pollTimer?.cancel();
    _pollTimer = Timer.periodic(interval, (_) => _poll());
  }

  /// Stop polling.
  void stopPolling() {
    _pollTimer?.cancel();
    _pollTimer = null;
  }

  /// Notify that a local write happened on [collection].
  void notifyLocal(String collection, SyncEventType type) {
    if (!_controller.isClosed) {
      _controller.add(SyncEvent(collection: collection, type: type));
    }
  }

  void _poll() {
    final current = _engine.syncVersion();
    if (current != _lastVersion) {
      _lastVersion = current;
      if (!_controller.isClosed) {
        _controller.add(
          const SyncEvent(collection: '*', type: SyncEventType.remoteChange),
        );
      }
    }
  }

  /// Creates a [Stream] that re-invokes [fetcher] whenever [collectionName]
  /// changes (or any global change occurs).
  ///
  /// If [fireImmediately] is true, [fetcher] is called once on listen.
  Stream<T> watch<T>(
    String collectionName,
    T Function() fetcher, {
    bool fireImmediately = true,
  }) {
    late StreamController<T> controller;
    StreamSubscription<SyncEvent>? sub;
    void requery() {
      if (!controller.isClosed) controller.add(fetcher());
    }

    controller = StreamController<T>(
      onListen: () {
        if (fireImmediately) requery();
        sub = changes.listen((event) {
          if (event.collection == '*' || event.collection == collectionName) {
            requery();
          }
        });
      },
      onCancel: () => sub?.cancel(),
    );
    return controller.stream;
  }

  /// Release resources.
  void dispose() {
    stopPolling();
    _controller.close();
  }
}
