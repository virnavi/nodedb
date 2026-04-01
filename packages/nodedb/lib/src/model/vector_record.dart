import '../util/msgpack.dart';

/// A vector record in a NodeDB vector store.
class VectorRecord {
  final int id;
  final List<double> vector;
  final Map<String, dynamic> metadata;

  const VectorRecord({
    required this.id,
    required this.vector,
    required this.metadata,
  });

  factory VectorRecord.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final rawVec = decodeField(decoded, 'vector', 1);
    final vector = (rawVec is List)
        ? rawVec.map((v) => (v as num).toDouble()).toList()
        : <double>[];
    final rawMeta = decodeField(decoded, 'metadata', 2);
    final metadata = rawMeta is Map
        ? Map<String, dynamic>.from(rawMeta)
        : <String, dynamic>{};
    return VectorRecord(id: id, vector: vector, metadata: metadata);
  }

  @override
  String toString() =>
      'VectorRecord(id: $id, dim: ${vector.length})';
}
