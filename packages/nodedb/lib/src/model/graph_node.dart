import '../util/msgpack.dart';

/// A node in a NodeDB graph.
class GraphNode {
  final int id;
  final String label;
  final Map<String, dynamic> data;

  const GraphNode({
    required this.id,
    required this.label,
    required this.data,
  });

  factory GraphNode.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final label = (decodeField(decoded, 'label', 1) ?? '') as String;
    final rawData = decodeField(decoded, 'data', 2);
    final data = rawData is Map
        ? Map<String, dynamic>.from(rawData)
        : <String, dynamic>{};
    return GraphNode(id: id, label: label, data: data);
  }

  @override
  String toString() => 'GraphNode(id: $id, label: $label)';
}
