/// Schema property types for AI query schema validation.
enum SchemaPropertyType {
  string,
  integer,
  float,
  boolean,
  array,
  map,
  any;

  /// Parse from a Rust FFI string.
  static SchemaPropertyType fromString(String s) {
    switch (s.toLowerCase()) {
      case 'string':
        return SchemaPropertyType.string;
      case 'integer':
        return SchemaPropertyType.integer;
      case 'float':
        return SchemaPropertyType.float;
      case 'boolean':
        return SchemaPropertyType.boolean;
      case 'array':
        return SchemaPropertyType.array;
      case 'map':
        return SchemaPropertyType.map;
      default:
        return SchemaPropertyType.any;
    }
  }
}

/// Context passed to the AI query adapter.
class AiQueryContext {
  /// Identity string for the AI subject (for DAC and provenance).
  final String aiSubjectId;

  /// Maximum results the adapter should return.
  final int maxResults;

  /// Minimum confidence for results to be persisted.
  final double minimumWriteConfidence;

  const AiQueryContext({
    this.aiSubjectId = 'ai-query-adapter',
    this.maxResults = 10,
    this.minimumWriteConfidence = 0.80,
  });
}

/// A single result returned by the AI query adapter.
class AiQueryResult {
  /// The record data to potentially persist.
  final Map<String, dynamic> data;

  /// Confidence score (0.0–1.0) from the AI.
  final double confidence;

  /// Human-readable explanation of the data source.
  final String sourceExplanation;

  /// Optional URI to the external data source.
  final String? externalSourceUri;

  /// Optional classification tags.
  final Map<String, String>? tags;

  const AiQueryResult({
    required this.data,
    required this.confidence,
    required this.sourceExplanation,
    this.externalSourceUri,
    this.tags,
  });

  /// Convert to the FFI-compatible map format for processResults.
  Map<String, dynamic> toFfiMap() => {
        'data': data,
        'confidence': confidence,
        'source_explanation': sourceExplanation,
        if (externalSourceUri != null) 'external_source_uri': externalSourceUri,
        if (tags != null) 'tags': tags,
      };
}

/// Decision made by the Rust engine about whether an AI query result was persisted.
class AiQueryWriteDecision {
  final bool persisted;
  final int? recordId;
  final double confidence;
  final String? aiOriginTag;
  final String? rejectionReason;

  const AiQueryWriteDecision({
    required this.persisted,
    this.recordId,
    required this.confidence,
    this.aiOriginTag,
    this.rejectionReason,
  });

  factory AiQueryWriteDecision.fromMap(Map<String, dynamic> m) {
    return AiQueryWriteDecision(
      persisted: m['persisted'] as bool? ?? false,
      recordId: m['record_id'] as int?,
      confidence: (m['confidence'] as num?)?.toDouble() ?? 0.0,
      aiOriginTag: m['ai_origin_tag'] as String?,
      rejectionReason: m['rejection_reason'] as String?,
    );
  }
}

/// Schema description for a collection, passed to the AI query adapter.
class AiQuerySchema {
  final List<String> requiredFields;
  final Map<String, SchemaPropertyType> fieldTypes;

  const AiQuerySchema({
    this.requiredFields = const [],
    this.fieldTypes = const {},
  });

  /// Convert to a JSON-compatible map.
  Map<String, dynamic> toMap() => {
        'required_fields': requiredFields,
        'field_types':
            fieldTypes.map((k, v) => MapEntry(k, v.name.capitalize())),
      };
}

/// Configuration for the AI query adapter.
class AiQueryConfig {
  /// Collections enabled for AI query fallback. Empty means none.
  final List<String> enabledCollections;

  /// Minimum confidence to persist a result. Default 0.80.
  final double minimumWriteConfidence;

  /// Maximum results per adapter call. Default 10.
  final int maxResultsPerQuery;

  /// Whether to report write decisions. Default true.
  final bool reportWriteDecisions;

  /// Whether to try federation before AI fallback. Default true.
  final bool tryFederationFirst;

  /// Maximum adapter calls per minute.
  final int rateLimitPerMinute;

  const AiQueryConfig({
    this.enabledCollections = const [],
    this.minimumWriteConfidence = 0.80,
    this.maxResultsPerQuery = 10,
    this.reportWriteDecisions = true,
    this.tryFederationFirst = true,
    this.rateLimitPerMinute = 20,
  });
}

extension _StringCap on String {
  String capitalize() =>
      isEmpty ? this : '${this[0].toUpperCase()}${substring(1)}';
}
