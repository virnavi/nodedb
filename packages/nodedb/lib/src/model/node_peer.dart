import '../util/msgpack.dart';

/// A peer in the NodeDB federation system.
class NodePeer {
  final int id;
  final String name;
  final String endpoint;
  final String status;
  final Map<String, dynamic>? metadata;

  const NodePeer({
    required this.id,
    required this.name,
    required this.endpoint,
    required this.status,
    this.metadata,
  });

  factory NodePeer.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final name = (decodeField(decoded, 'name', 1) ?? '') as String;
    final endpoint = (decodeField(decoded, 'endpoint', 2) ?? '') as String;
    final status = (decodeField(decoded, 'status', 3) ?? 'active') as String;
    final rawMeta = decodeField(decoded, 'metadata', 4);
    final metadata = rawMeta is Map
        ? Map<String, dynamic>.from(rawMeta)
        : null;
    return NodePeer(
      id: id,
      name: name,
      endpoint: endpoint,
      status: status,
      metadata: metadata,
    );
  }

  @override
  String toString() => 'NodePeer(id: $id, name: $name, status: $status)';
}
