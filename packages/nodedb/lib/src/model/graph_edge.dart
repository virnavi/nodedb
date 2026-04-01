import '../util/msgpack.dart';

/// An edge in a NodeDB graph.
class GraphEdge {
  final int id;
  final String label;
  final int source;
  final int target;
  final double weight;
  final Map<String, dynamic> data;

  const GraphEdge({
    required this.id,
    required this.label,
    required this.source,
    required this.target,
    required this.weight,
    required this.data,
  });

  factory GraphEdge.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final label = (decodeField(decoded, 'label', 1) ?? '') as String;
    final source = decodeField(decoded, 'source', 2) as int;
    final target = decodeField(decoded, 'target', 3) as int;
    final w = decodeField(decoded, 'weight', 4);
    final weight = (w is double) ? w : (w as num).toDouble();
    final rawData = decodeField(decoded, 'data', 5);
    final data = rawData is Map
        ? Map<String, dynamic>.from(rawData)
        : <String, dynamic>{};
    return GraphEdge(
      id: id,
      label: label,
      source: source,
      target: target,
      weight: weight,
      data: data,
    );
  }

  @override
  String toString() =>
      'GraphEdge(id: $id, label: $label, $source -> $target, weight: $weight)';
}
