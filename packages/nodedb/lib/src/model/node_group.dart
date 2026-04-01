import '../util/msgpack.dart';

/// A group in the NodeDB federation system.
class NodeGroup {
  final int id;
  final String name;
  final List<int> members;
  final Map<String, dynamic>? metadata;

  const NodeGroup({
    required this.id,
    required this.name,
    required this.members,
    this.metadata,
  });

  factory NodeGroup.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final name = (decodeField(decoded, 'name', 1) ?? '') as String;
    final rawMembers = decodeField(decoded, 'members', 2);
    final members = (rawMembers is List)
        ? rawMembers.map((v) => v as int).toList()
        : <int>[];
    final rawMeta = decodeField(decoded, 'metadata', 3);
    final metadata = rawMeta is Map
        ? Map<String, dynamic>.from(rawMeta)
        : null;
    return NodeGroup(
      id: id,
      name: name,
      members: members,
      metadata: metadata,
    );
  }

  @override
  String toString() => 'NodeGroup(id: $id, name: $name, members: $members)';
}
