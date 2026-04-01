import '../engine/nosql_engine.dart';
import '../model/document.dart';

/// Built-in mesh-level store for P2P async request/response messages.
///
/// Each message has a [requestId] that links a request to its response.
/// Messages carry a [validUntil] timestamp — expired messages are
/// automatically swept by [sweepExpired].
///
/// This store lives inside [DatabaseMesh]'s internal mesh database
/// and is accessible to the background P2P service isolate.
class P2pMessageStore {
  final NoSqlEngine _engine;

  /// The collection name used for P2P messages.
  static const collection = 'p2p_messages';

  P2pMessageStore(this._engine);

  // ── Write ──────────────────────────────────────────────────────

  /// Create a request message. Returns the sled record ID.
  int createRequest({
    required String requestId,
    required String fromPeerId,
    required String toPeerId,
    required String action,
    required Map<String, dynamic> data,
    required DateTime validUntil,
  }) {
    return _put({
      'requestId': requestId,
      'type': 'request',
      'fromPeerId': fromPeerId,
      'toPeerId': toPeerId,
      'action': action,
      'data': data,
      'validUntil': validUntil.toIso8601String(),
      'status': 'pending',
      'createdAt': DateTime.now().toUtc().toIso8601String(),
    });
  }

  /// Create a response message. Returns the sled record ID.
  int createResponse({
    required String requestId,
    required String fromPeerId,
    required String toPeerId,
    required String action,
    required Map<String, dynamic> data,
    required DateTime validUntil,
  }) {
    return _put({
      'requestId': requestId,
      'type': 'response',
      'fromPeerId': fromPeerId,
      'toPeerId': toPeerId,
      'action': action,
      'data': data,
      'validUntil': validUntil.toIso8601String(),
      'status': 'pending',
      'createdAt': DateTime.now().toUtc().toIso8601String(),
    });
  }

  /// Create a lightweight acknowledgement message. Returns the sled record ID.
  int createAck({
    required String requestId,
    required String fromPeerId,
    required String toPeerId,
  }) {
    return _put({
      'requestId': requestId,
      'type': 'ack',
      'fromPeerId': fromPeerId,
      'toPeerId': toPeerId,
      'status': 'pending',
      'validUntil':
          DateTime.now().toUtc().add(const Duration(minutes: 2)).toIso8601String(),
      'createdAt': DateTime.now().toUtc().toIso8601String(),
    });
  }

  // ── Read ───────────────────────────────────────────────────────

  /// Get all messages.
  List<Document> findAll() => _engine.findAll(collection);

  /// Get a single message by sled ID.
  Document? findById(int id) => _engine.get(collection, id);

  /// Find all messages matching a request ID (request + its response).
  List<Document> findByRequestId(String requestId) {
    return _engine.findAll(collection).where((d) {
      return d.data['requestId'] == requestId;
    }).toList();
  }

  /// Find pending requests addressed to [peerId].
  List<Document> findPendingRequestsTo(String peerId) {
    return _engine.findAll(collection).where((d) {
      return d.data['type'] == 'request' &&
          d.data['toPeerId'] == peerId &&
          d.data['status'] == 'pending';
    }).toList();
  }

  /// Find pending responses for a given [requestId].
  List<Document> findPendingResponsesFor(String requestId) {
    return _engine.findAll(collection).where((d) {
      return d.data['type'] == 'response' &&
          d.data['requestId'] == requestId &&
          d.data['status'] == 'pending';
    }).toList();
  }

  /// Find all outgoing requests from [peerId] that are still pending.
  List<Document> findPendingOutgoing(String peerId) {
    return _engine.findAll(collection).where((d) {
      return d.data['type'] == 'request' &&
          d.data['fromPeerId'] == peerId &&
          d.data['status'] == 'pending';
    }).toList();
  }

  // ── Status updates ─────────────────────────────────────────────

  void markDelivered(int id) => _updateStatus(id, 'delivered');
  void markProcessed(int id) => _updateStatus(id, 'processed');
  void markRejected(int id) => _updateStatus(id, 'rejected');
  void markTimeout(int id) => _updateStatus(id, 'timeout');

  void _updateStatus(int id, String status) {
    final doc = _engine.get(collection, id);
    if (doc == null) return;
    final updated = Map<String, dynamic>.from(doc.data);
    updated['status'] = status;
    _engine.writeTxn([WriteOp.put(collection, data: updated, id: id)]);
  }

  // ── Delete ─────────────────────────────────────────────────────

  void deleteById(int id) {
    _engine.writeTxn([WriteOp.delete(collection, id: id)]);
  }

  // ── Auto-trim ──────────────────────────────────────────────────

  /// Delete all messages past their [validUntil] timestamp.
  /// Returns the number of deleted messages and a list of timed-out
  /// outgoing request IDs (for notification).
  SweepResult sweepExpired() {
    final now = DateTime.now().toUtc();
    final all = _engine.findAll(collection);
    final toDelete = <int>[];
    final timedOutRequestIds = <String>[];

    for (final doc in all) {
      final validStr = doc.data['validUntil'];
      if (validStr is! String) continue;
      final validUntil = DateTime.tryParse(validStr);
      if (validUntil == null) continue;

      if (now.isAfter(validUntil)) {
        // Track timed-out outgoing requests for caller notification
        if (doc.data['type'] == 'request' &&
            doc.data['status'] == 'pending') {
          final reqId = doc.data['requestId'];
          if (reqId is String) timedOutRequestIds.add(reqId);
        }
        toDelete.add(doc.id);
      }
    }

    if (toDelete.isNotEmpty) {
      _engine.batchDelete(collection, toDelete);
    }

    return SweepResult(
      deletedCount: toDelete.length,
      timedOutRequestIds: timedOutRequestIds,
    );
  }

  /// Total message count.
  int count() => _engine.count(collection);

  // ── Internal ───────────────────────────────────────────────────

  int _put(Map<String, dynamic> data) {
    // Use batchPut for auto-ID assignment and get the count back.
    // We write a single item; the sled engine auto-assigns an ID.
    _engine.writeTxn([WriteOp.put(collection, data: data)]);
    // Return the latest document's id (the one we just wrote).
    // For simplicity, find all and get the last one.
    final all = _engine.findAll(collection);
    return all.isNotEmpty ? all.last.id : 0;
  }
}

/// Result of [P2pMessageStore.sweepExpired].
class SweepResult {
  final int deletedCount;
  final List<String> timedOutRequestIds;

  const SweepResult({
    required this.deletedCount,
    required this.timedOutRequestIds,
  });
}
