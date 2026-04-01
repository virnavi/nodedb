/// Fluent query builder that constructs filter/sort/offset/limit maps
/// matching NodeDB's Rust `FilterCondition` enum for FFI transport.
class FilterQuery<T> {
  final List<Map<String, dynamic>> _conditions = [];
  final List<List<Map<String, dynamic>>> _orGroups = [];
  final List<Map<String, dynamic>> _sort = [];
  int? _offset;
  int? _limit;
  bool _distinct = false;
  bool _withProvenance = false;
  bool _withFederation = false;
  bool _withAiQuery = false;

  FilterQuery();

  // ── Condition builders ──────────────────────────────────────────

  FilterQuery<T> equalTo(String field, dynamic value) {
    _conditions.add({'EqualTo': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> notEqualTo(String field, dynamic value) {
    _conditions.add({'NotEqualTo': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> greaterThan(String field, dynamic value) {
    _conditions.add({'GreaterThan': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> greaterThanOrEqual(String field, dynamic value) {
    _conditions.add({'GreaterThanOrEqual': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> lessThan(String field, dynamic value) {
    _conditions.add({'LessThan': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> lessThanOrEqual(String field, dynamic value) {
    _conditions.add({'LessThanOrEqual': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> contains(String field, String value) {
    _conditions.add({'Contains': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> startsWith(String field, String value) {
    _conditions.add({'StartsWith': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> endsWith(String field, String value) {
    _conditions.add({'EndsWith': {'field': field, 'value': value}});
    return this;
  }

  FilterQuery<T> isNull(String field) {
    _conditions.add({'IsNull': {'field': field}});
    return this;
  }

  FilterQuery<T> isNotNull(String field) {
    _conditions.add({'IsNotNull': {'field': field}});
    return this;
  }

  FilterQuery<T> between(String field, dynamic low, dynamic high) {
    _conditions.add({
      'Between': {'field': field, 'low': low, 'high': high},
    });
    return this;
  }

  // ── JSONB / Array operators ───────────────────────────────────

  /// Match a value at a JSON path within a JSONB field.
  FilterQuery<T> jsonPathEquals(String field, String path, dynamic value) {
    _conditions.add({
      'JsonPathEquals': {'field': field, 'path': path, 'value': value},
    });
    return this;
  }

  /// Check if a key exists (and is not null) at a JSON path.
  FilterQuery<T> jsonHasKey(String field, String path) {
    _conditions.add({'JsonHasKey': {'field': field, 'path': path}});
    return this;
  }

  /// Check if a Map field contains all entries from the given map.
  FilterQuery<T> jsonContains(String field, Map<String, dynamic> value) {
    _conditions.add({'JsonContains': {'field': field, 'value': value}});
    return this;
  }

  /// Check if an array field contains a specific element.
  FilterQuery<T> arrayContains(String field, dynamic value) {
    _conditions.add({'ArrayContains': {'field': field, 'value': value}});
    return this;
  }

  /// Check if an array field has any overlap with the given values.
  FilterQuery<T> arrayOverlap(String field, List<dynamic> values) {
    _conditions.add({'ArrayOverlap': {'field': field, 'values': values}});
    return this;
  }

  /// Match if field value is in the given list.
  ///
  /// Expands to `Or([EqualTo(field, v1), EqualTo(field, v2), ...])`.
  FilterQuery<T> inList(String field, List<dynamic> values) {
    if (values.isEmpty) return this;
    if (values.length == 1) return equalTo(field, values.first);
    // Save current conditions as one group, add OR group for each value
    if (_conditions.isNotEmpty) {
      _orGroups.add(List.from(_conditions));
      _conditions.clear();
    }
    for (final v in values) {
      _orGroups.add([{'EqualTo': {'field': field, 'value': v}}]);
    }
    return this;
  }

  /// Match if field value is NOT in the given list.
  ///
  /// Expands to `And([NotEqualTo(field, v1), NotEqualTo(field, v2), ...])`.
  FilterQuery<T> notInList(String field, List<dynamic> values) {
    for (final v in values) {
      _conditions.add({'NotEqualTo': {'field': field, 'value': v}});
    }
    return this;
  }

  // ── Combinators ─────────────────────────────────────────────────

  /// Combine current conditions with another group via AND.
  ///
  /// ```dart
  /// query.equalTo('age', 25).and((q) => q.equalTo('active', true));
  /// ```
  FilterQuery<T> and(FilterQuery<T> Function(FilterQuery<T>) builder) {
    final sub = builder(FilterQuery<T>());
    // Merge sub's conditions into current group (AND is default)
    _conditions.addAll(sub._conditions);
    return this;
  }

  /// Start an OR group.
  ///
  /// ```dart
  /// query.equalTo('role', 'admin').or((q) => q.equalTo('role', 'superadmin'));
  /// ```
  FilterQuery<T> or(FilterQuery<T> Function(FilterQuery<T>) builder) {
    final sub = builder(FilterQuery<T>());
    // Save current conditions as one AND group, start OR
    if (_conditions.isNotEmpty) {
      _orGroups.add(List.from(_conditions));
      _conditions.clear();
    }
    _orGroups.add(sub._conditions);
    return this;
  }

  // ── Query flags ────────────────────────────────────────────────

  /// Deduplicate results by document ID (Dart-side).
  FilterQuery<T> distinct() {
    _distinct = true;
    return this;
  }

  /// Attach provenance envelopes to each result.
  FilterQuery<T> withProvenance() {
    _withProvenance = true;
    return this;
  }

  /// Query federated peers in addition to local data.
  FilterQuery<T> withFederation() {
    _withFederation = true;
    return this;
  }

  /// Alias for [withFederation] — query across all reachable peers.
  FilterQuery<T> acrossPeers() => withFederation();

  /// Fall back to AI query adapter when local results are empty.
  FilterQuery<T> withAiQuery() {
    _withAiQuery = true;
    return this;
  }

  /// Whether this query requests distinct (deduplicated) results.
  bool get isDistinct => _distinct;

  /// Whether this query requests provenance envelopes.
  bool get isWithProvenance => _withProvenance;

  /// Whether this query requests federated peer results.
  bool get isWithFederation => _withFederation;

  /// Whether this query requests AI query fallback.
  bool get isWithAiQuery => _withAiQuery;

  // ── Sorting / Pagination ────────────────────────────────────────

  FilterQuery<T> sortBy(String field, {bool desc = false}) {
    _sort.add({
      'field': field,
      'direction': desc ? 'Desc' : 'Asc',
    });
    return this;
  }

  FilterQuery<T> offset(int value) {
    _offset = value;
    return this;
  }

  FilterQuery<T> limit(int value) {
    _limit = value;
    return this;
  }

  // ── Build ───────────────────────────────────────────────────────

  /// Build the filter map for FFI transport.
  ///
  /// Produces the structure expected by `NoSqlEngine.findAll()`:
  /// `{'filter': {...}, 'sort': [...], 'offset': n, 'limit': n}`
  Map<String, dynamic> build() {
    final result = <String, dynamic>{};

    final filter = _buildFilter();
    if (filter != null) result['filter'] = filter;
    if (_sort.isNotEmpty) result['sort'] = _sort;
    if (_offset != null) result['offset'] = _offset;
    if (_limit != null) result['limit'] = _limit;
    if (_distinct) result['distinct'] = true;

    return result;
  }

  Map<String, dynamic>? _buildFilter() {
    if (_orGroups.isEmpty && _conditions.isEmpty) return null;

    // If we have OR groups, wrap everything as Or([And([...]), And([...]), ...])
    if (_orGroups.isNotEmpty) {
      final groups = [..._orGroups];
      if (_conditions.isNotEmpty) {
        groups.add(List.from(_conditions));
      }

      final orFilters = groups.map((group) {
        if (group.length == 1) {
          return {'Condition': group.first};
        }
        return {
          'And': group.map((c) => {'Condition': c}).toList(),
        };
      }).toList();

      if (orFilters.length == 1) return orFilters.first;
      return {'Or': orFilters};
    }

    // All conditions ANDed together
    if (_conditions.length == 1) {
      return {'Condition': _conditions.first};
    }

    return {
      'And': _conditions.map((c) => {'Condition': c}).toList(),
    };
  }

  // ── Describe ─────────────────────────────────────────────────

  /// Generate a human-readable description of this query.
  ///
  /// Useful for AI adapter context and debugging.
  String describe() {
    final parts = <String>[];

    // Describe conditions
    final allConditions = <Map<String, dynamic>>[
      ..._conditions,
      for (final group in _orGroups) ...group,
    ];
    if (allConditions.isNotEmpty) {
      final descs = allConditions.map(_describeCondition).toList();
      if (_orGroups.isNotEmpty) {
        parts.add('where ${descs.join(' OR ')}');
      } else {
        parts.add('where ${descs.join(' AND ')}');
      }
    }

    // Describe sort
    for (final s in _sort) {
      final dir = s['direction'] == 'Desc' ? 'descending' : 'ascending';
      parts.add('sorted by ${s['field']} $dir');
    }

    // Describe pagination
    if (_offset != null) parts.add('offset $_offset');
    if (_limit != null) parts.add('limit $_limit');
    if (_distinct) parts.add('distinct');

    return parts.isEmpty ? 'all records' : parts.join(', ');
  }

  static String _describeCondition(Map<String, dynamic> condition) {
    final entry = condition.entries.first;
    final op = entry.key;
    final params = entry.value as Map<String, dynamic>;
    final field = params['field'];

    switch (op) {
      case 'EqualTo':
        return '$field = ${params['value']}';
      case 'NotEqualTo':
        return '$field != ${params['value']}';
      case 'GreaterThan':
        return '$field > ${params['value']}';
      case 'GreaterThanOrEqual':
        return '$field >= ${params['value']}';
      case 'LessThan':
        return '$field < ${params['value']}';
      case 'LessThanOrEqual':
        return '$field <= ${params['value']}';
      case 'Contains':
        return '$field contains "${params['value']}"';
      case 'StartsWith':
        return '$field starts with "${params['value']}"';
      case 'EndsWith':
        return '$field ends with "${params['value']}"';
      case 'IsNull':
        return '$field is null';
      case 'IsNotNull':
        return '$field is not null';
      case 'Between':
        return '$field between ${params['low']} and ${params['high']}';
      case 'JsonPathEquals':
        return '$field->${params['path']} = ${params['value']}';
      case 'JsonHasKey':
        return '$field has key ${params['path']}';
      case 'JsonContains':
        return '$field @> ${params['value']}';
      case 'ArrayContains':
        return '$field contains ${params['value']}';
      case 'ArrayOverlap':
        return '$field overlaps ${params['values']}';
      default:
        return '$op on $field';
    }
  }
}
