import '../model/ai_provenance.dart';
import '../model/provenance_envelope.dart';

/// Abstract adapter for AI-powered provenance augmentation.
///
/// Implement this class to connect an external AI service (e.g., OpenAI,
/// local LLM) to NodeDB's provenance system. The adapter is called by the
/// database to assess records, resolve conflicts, detect anomalies, and
/// classify data sources.
///
/// ```dart
/// class MyAiAdapter extends NodeDbAiProvenanceAdapter {
///   @override
///   Future<AiProvenanceAssessment?> assessRecord({...}) async {
///     // Call your AI service here
///     return AiProvenanceAssessment(suggestedConfidence: 0.85);
///   }
///   // ... implement other methods
/// }
/// ```
abstract class NodeDbAiProvenanceAdapter {
  /// Assess a record and suggest confidence/metadata adjustments.
  ///
  /// Called when a record is written or updated. The [recordJson] is
  /// DAC-filtered (denied fields removed). Return null to skip augmentation.
  Future<AiProvenanceAssessment?> assessRecord({
    required String collection,
    required String recordJson,
    required ProvenanceEnvelope currentEnvelope,
  });

  /// Resolve a conflict between two provenance envelopes.
  ///
  /// Called when corroborating data sources disagree. Return null to
  /// use the default resolution strategy.
  Future<AiConflictResolution?> resolveConflict({
    required String collection,
    required ProvenanceEnvelope envelopeA,
    required ProvenanceEnvelope envelopeB,
    required String recordAJson,
    required String recordBJson,
  });

  /// Detect anomalies across a set of provenance envelopes.
  ///
  /// Called periodically based on [AiProvenanceConfig.anomalyDetectionInterval].
  /// Return anomaly flags for records that appear suspicious.
  Future<List<AiAnomalyFlag>> detectAnomalies({
    required String collection,
    required List<ProvenanceEnvelope> envelopes,
  });

  /// Classify a data source and estimate its credibility.
  ///
  /// Called for new or unknown source IDs. Return null to use default
  /// source type handling.
  Future<AiSourceClassification?> classifySource({
    required String rawSourceId,
    required String? context,
  });
}
