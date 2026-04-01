import 'package:nodedb/nodedb.dart';
import 'package:test/test.dart';

/// Matches a [Document] with the given [id].
Matcher hasDocumentId(int id) => _HasDocumentId(id);

/// Matches a [Document] whose data contains all key-value pairs from [data].
Matcher hasDocumentData(Map<String, dynamic> data) => _HasDocumentData(data);

/// Matches a [Document] with the given [id] and data containing [data].
Matcher isDocument({int? id, Map<String, dynamic>? data}) =>
    _IsDocument(id: id, data: data);

/// Matches a list of [Document]s with the given length.
Matcher hasDocumentCount(int count) => hasLength(count);

/// Matches a [GraphNode] with the given [label].
Matcher hasNodeLabel(String label) => _HasNodeLabel(label);

/// Matches a [GraphEdge] connecting [source] to [target].
Matcher connectsNodes(int source, int target) =>
    _ConnectsNodes(source, target);

class _HasDocumentId extends Matcher {
  final int _id;
  _HasDocumentId(this._id);

  @override
  bool matches(Object? item, Map matchState) =>
      item is Document && item.id == _id;

  @override
  Description describe(Description description) =>
      description.add('Document with id=$_id');
}

class _HasDocumentData extends Matcher {
  final Map<String, dynamic> _data;
  _HasDocumentData(this._data);

  @override
  bool matches(Object? item, Map matchState) {
    if (item is! Document) return false;
    for (final entry in _data.entries) {
      if (item.data[entry.key] != entry.value) return false;
    }
    return true;
  }

  @override
  Description describe(Description description) =>
      description.add('Document with data containing $_data');
}

class _IsDocument extends Matcher {
  final int? _id;
  final Map<String, dynamic>? _data;
  _IsDocument({int? id, Map<String, dynamic>? data}) : _id = id, _data = data;

  @override
  bool matches(Object? item, Map matchState) {
    if (item is! Document) return false;
    if (_id != null && item.id != _id) return false;
    final data = _data;
    if (data != null) {
      for (final entry in data.entries) {
        if (item.data[entry.key] != entry.value) return false;
      }
    }
    return true;
  }

  @override
  Description describe(Description description) {
    description.add('Document');
    if (_id != null) description.add(' with id=$_id');
    if (_data != null) description.add(' with data containing $_data');
    return description;
  }
}

class _HasNodeLabel extends Matcher {
  final String _label;
  _HasNodeLabel(this._label);

  @override
  bool matches(Object? item, Map matchState) =>
      item is GraphNode && item.label == _label;

  @override
  Description describe(Description description) =>
      description.add('GraphNode with label=$_label');
}

class _ConnectsNodes extends Matcher {
  final int _source;
  final int _target;
  _ConnectsNodes(this._source, this._target);

  @override
  bool matches(Object? item, Map matchState) =>
      item is GraphEdge && item.source == _source && item.target == _target;

  @override
  Description describe(Description description) =>
      description.add('GraphEdge connecting $_source -> $_target');
}
