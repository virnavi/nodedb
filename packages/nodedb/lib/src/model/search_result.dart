import '../util/msgpack.dart';

/// A vector search result with distance score.
class SearchResult {
  final int id;
  final double distance;
  final Map<String, dynamic> metadata;

  const SearchResult({
    required this.id,
    required this.distance,
    required this.metadata,
  });

  factory SearchResult.fromMsgpack(dynamic decoded) {
    final id = decodeField(decoded, 'id', 0) as int;
    final d = decodeField(decoded, 'distance', 1);
    final distance = (d is double) ? d : (d as num).toDouble();
    final rawMeta = decodeField(decoded, 'metadata', 2);
    final metadata = rawMeta is Map
        ? Map<String, dynamic>.from(rawMeta)
        : <String, dynamic>{};
    return SearchResult(id: id, distance: distance, metadata: metadata);
  }

  @override
  String toString() => 'SearchResult(id: $id, distance: $distance)';
}
