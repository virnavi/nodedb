/// AI conflict resolution preference.
enum ConflictPreference {
  preferA,
  preferB,
  indeterminate;

  /// Convert to the Rust FFI string representation.
  String toFfiString() {
    switch (this) {
      case ConflictPreference.preferA:
        return 'prefer_a';
      case ConflictPreference.preferB:
        return 'prefer_b';
      case ConflictPreference.indeterminate:
        return 'prefer_neither';
    }
  }

  /// Parse from a Rust FFI string.
  static ConflictPreference fromFfiString(String s) {
    switch (s) {
      case 'prefer_a':
      case 'PreferA':
        return ConflictPreference.preferA;
      case 'prefer_b':
      case 'PreferB':
        return ConflictPreference.preferB;
      default:
        return ConflictPreference.indeterminate;
    }
  }
}

/// Severity level for AI-detected anomalies.
enum AnomalySeverity {
  low,
  medium,
  high,
  critical;

  /// Parse from a string.
  static AnomalySeverity fromString(String s) {
    switch (s.toLowerCase()) {
      case 'low':
        return AnomalySeverity.low;
      case 'medium':
        return AnomalySeverity.medium;
      case 'high':
        return AnomalySeverity.high;
      case 'critical':
        return AnomalySeverity.critical;
      default:
        return AnomalySeverity.low;
    }
  }
}

/// Result of an AI provenance assessment on a record.
class AiProvenanceAssessment {
  final double? suggestedConfidence;
  final String? sourceType;
  final String? reasoning;
  final Map<String, String>? tags;

  const AiProvenanceAssessment({
    this.suggestedConfidence,
    this.sourceType,
    this.reasoning,
    this.tags,
  });

  /// Convert to a map suitable for the FFI engine.
  Map<String, dynamic> toFfiMap(int envelopeId) => {
        'envelope_id': envelopeId,
        if (suggestedConfidence != null)
          'suggested_confidence': suggestedConfidence,
        if (sourceType != null) 'source_type': sourceType,
        if (reasoning != null) 'reasoning': reasoning,
        if (tags != null) 'tags': tags,
      };
}

/// Result of an AI conflict resolution between two provenance envelopes.
class AiConflictResolution {
  final double confidenceDeltaA;
  final double confidenceDeltaB;
  final ConflictPreference preference;
  final String? reasoning;

  const AiConflictResolution({
    required this.confidenceDeltaA,
    required this.confidenceDeltaB,
    this.preference = ConflictPreference.indeterminate,
    this.reasoning,
  });
}

/// An AI-detected anomaly flag for a specific record.
class AiAnomalyFlag {
  final int recordId;
  final double confidencePenalty;
  final String? reason;
  final AnomalySeverity severity;

  const AiAnomalyFlag({
    required this.recordId,
    required this.confidencePenalty,
    this.reason,
    this.severity = AnomalySeverity.low,
  });

  /// Convert to a map for the FFI engine.
  Map<String, dynamic> toFfiMap() => {
        'record_id': recordId,
        'confidence_penalty': confidencePenalty,
        if (reason != null) 'reason': reason,
        'severity': severity.name,
      };
}

/// Result of an AI source classification.
class AiSourceClassification {
  final String sourceType;
  final double credibilityPrior;
  final String? reasoning;

  const AiSourceClassification({
    required this.sourceType,
    required this.credibilityPrior,
    this.reasoning,
  });
}

/// Configuration for the AI provenance adapter.
class AiProvenanceConfig {
  /// Blend weight for AI confidence (0.0–1.0). Default 0.3.
  final double aiBlendWeight;

  /// Collections the adapter is enabled for. Empty means all.
  final List<String> enabledCollections;

  /// How often anomaly detection runs automatically.
  final Duration anomalyDetectionInterval;

  /// Timeout for adapter responses.
  final Duration responseTimeout;

  /// If true, adapter errors are silently ignored.
  final bool silentOnError;

  /// Maximum adapter calls per minute.
  final int rateLimitPerMinute;

  const AiProvenanceConfig({
    this.aiBlendWeight = 0.3,
    this.enabledCollections = const [],
    this.anomalyDetectionInterval = const Duration(minutes: 30),
    this.responseTimeout = const Duration(seconds: 5),
    this.silentOnError = true,
    this.rateLimitPerMinute = 60,
  });
}
