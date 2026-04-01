/// Trim policy types.
///
/// All collections are never-trim by default. Trimming must be explicitly opted into.
sealed class TrimPolicy {
  const TrimPolicy();

  /// Convert to a map for FFI serialization.
  Map<String, dynamic> toMap();

  /// Parse from a map (from FFI).
  static TrimPolicy fromMap(Map<String, dynamic> map) {
    final type = map['type'] as String;
    switch (type) {
      case 'not_accessed_since':
        return TrimNotAccessedSince(map['duration_secs'] as int);
      case 'not_read_since':
        return TrimNotReadSince(map['duration_secs'] as int);
      case 'ai_originated_only':
        return const TrimAiOriginatedOnly();
      case 'confidence_below':
        return TrimConfidenceBelow((map['threshold'] as num).toDouble());
      case 'to_target_bytes':
        return TrimToTargetBytes(map['max_bytes'] as int);
      case 'keep_most_recently_accessed':
        return TrimKeepMostRecentlyAccessed(map['count'] as int);
      case 'compound':
        final policies = (map['policies'] as List)
            .map((p) => TrimPolicy.fromMap(Map<String, dynamic>.from(p as Map)))
            .toList();
        return TrimCompound(policies);
      case 'any':
        final policies = (map['policies'] as List)
            .map((p) => TrimPolicy.fromMap(Map<String, dynamic>.from(p as Map)))
            .toList();
        return TrimAny(policies);
      default:
        throw ArgumentError('Unknown trim policy type: $type');
    }
  }
}

/// Trim records not accessed since the given duration.
class TrimNotAccessedSince extends TrimPolicy {
  final int durationSecs;
  const TrimNotAccessedSince(this.durationSecs);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'not_accessed_since',
        'duration_secs': durationSecs,
      };
}

/// Trim records not read since the given duration.
class TrimNotReadSince extends TrimPolicy {
  final int durationSecs;
  const TrimNotReadSince(this.durationSecs);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'not_read_since',
        'duration_secs': durationSecs,
      };
}

/// Only trim AI-originated records.
class TrimAiOriginatedOnly extends TrimPolicy {
  const TrimAiOriginatedOnly();

  @override
  Map<String, dynamic> toMap() => {'type': 'ai_originated_only'};
}

/// Trim records with confidence below threshold.
class TrimConfidenceBelow extends TrimPolicy {
  final double threshold;
  const TrimConfidenceBelow(this.threshold);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'confidence_below',
        'threshold': threshold,
      };
}

/// Trim LRU records until storage is within target bytes.
class TrimToTargetBytes extends TrimPolicy {
  final int maxBytes;
  const TrimToTargetBytes(this.maxBytes);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'to_target_bytes',
        'max_bytes': maxBytes,
      };
}

/// Keep only the N most recently accessed records per collection.
class TrimKeepMostRecentlyAccessed extends TrimPolicy {
  final int count;
  const TrimKeepMostRecentlyAccessed(this.count);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'keep_most_recently_accessed',
        'count': count,
      };
}

/// All sub-policies must match (AND).
class TrimCompound extends TrimPolicy {
  final List<TrimPolicy> policies;
  const TrimCompound(this.policies);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'compound',
        'policies': policies.map((p) => p.toMap()).toList(),
      };
}

/// At least one sub-policy must match (OR).
class TrimAny extends TrimPolicy {
  final List<TrimPolicy> policies;
  const TrimAny(this.policies);

  @override
  Map<String, dynamic> toMap() => {
        'type': 'any',
        'policies': policies.map((p) => p.toMap()).toList(),
      };
}

/// A candidate record for trimming.
class TrimCandidate {
  final String collection;
  final int recordId;
  final String? lastAccessedAtUtc;
  final int? ageSinceLastAccessSecs;
  final bool aiOriginated;
  final double? confidence;
  final bool neverTrimProtected;
  final List<String> reasons;

  const TrimCandidate({
    required this.collection,
    required this.recordId,
    this.lastAccessedAtUtc,
    this.ageSinceLastAccessSecs,
    this.aiOriginated = false,
    this.confidence,
    this.neverTrimProtected = false,
    this.reasons = const [],
  });

  factory TrimCandidate.fromMap(Map<String, dynamic> map) {
    return TrimCandidate(
      collection: map['collection'] as String? ?? '',
      recordId: map['record_id'] as int? ?? 0,
      lastAccessedAtUtc: map['last_accessed_at_utc'] as String?,
      ageSinceLastAccessSecs: map['age_since_last_access_secs'] as int?,
      aiOriginated: map['ai_originated'] as bool? ?? false,
      confidence: (map['confidence'] as num?)?.toDouble(),
      neverTrimProtected: map['never_trim_protected'] as bool? ?? false,
      reasons: (map['reasons'] as List?)?.cast<String>() ?? [],
    );
  }
}

/// Per-collection recommendation within a TrimRecommendation.
class TrimCollectionRecommendation {
  final String collection;
  final int candidateCount;
  final List<TrimCandidate> candidates;

  const TrimCollectionRecommendation({
    required this.collection,
    required this.candidateCount,
    required this.candidates,
  });

  factory TrimCollectionRecommendation.fromMap(Map<String, dynamic> map) {
    return TrimCollectionRecommendation(
      collection: map['collection'] as String? ?? '',
      candidateCount: map['candidate_count'] as int? ?? 0,
      candidates: (map['candidates'] as List?)
              ?.map((c) => TrimCandidate.fromMap(Map<String, dynamic>.from(c as Map)))
              .toList() ??
          [],
    );
  }
}

/// Result of recommendTrim — a full trim recommendation.
class TrimRecommendation {
  final int totalCandidateCount;
  final List<TrimCollectionRecommendation> byCollection;
  final String generatedAtUtc;

  const TrimRecommendation({
    required this.totalCandidateCount,
    required this.byCollection,
    required this.generatedAtUtc,
  });

  factory TrimRecommendation.fromMap(Map<String, dynamic> map) {
    return TrimRecommendation(
      totalCandidateCount: map['total_candidate_count'] as int? ?? 0,
      byCollection: (map['by_collection'] as List?)
              ?.map((c) =>
                  TrimCollectionRecommendation.fromMap(Map<String, dynamic>.from(c as Map)))
              .toList() ??
          [],
      generatedAtUtc: map['generated_at_utc'] as String? ?? '',
    );
  }
}

/// Report of a completed trim operation.
class TrimReport {
  final String collection;
  final int candidateCount;
  final int deletedCount;
  final int skippedCount;
  final int neverTrimSkippedCount;
  final int triggerAbortedCount;
  final bool dryRun;
  final String executedAtUtc;
  final List<Map<String, dynamic>> deletedRecordIds;

  const TrimReport({
    required this.collection,
    required this.candidateCount,
    required this.deletedCount,
    required this.skippedCount,
    required this.neverTrimSkippedCount,
    required this.triggerAbortedCount,
    required this.dryRun,
    required this.executedAtUtc,
    required this.deletedRecordIds,
  });

  factory TrimReport.fromMap(Map<String, dynamic> map) {
    return TrimReport(
      collection: map['collection'] as String? ?? '',
      candidateCount: map['candidate_count'] as int? ?? 0,
      deletedCount: map['deleted_count'] as int? ?? 0,
      skippedCount: map['skipped_count'] as int? ?? 0,
      neverTrimSkippedCount: map['never_trim_skipped_count'] as int? ?? 0,
      triggerAbortedCount: map['trigger_aborted_count'] as int? ?? 0,
      dryRun: map['dry_run'] as bool? ?? false,
      executedAtUtc: map['executed_at_utc'] as String? ?? '',
      deletedRecordIds: (map['deleted_record_ids'] as List?)
              ?.map((e) => Map<String, dynamic>.from(e as Map))
              .toList() ??
          [],
    );
  }
}

/// User-approved trim request.
class UserApprovedTrim {
  final TrimPolicy policy;
  final List<({String collection, int recordId})> confirmedRecordIds;
  final String? approvalNote;

  const UserApprovedTrim({
    required this.policy,
    required this.confirmedRecordIds,
    this.approvalNote,
  });

  Map<String, dynamic> toMap() => {
        'policy': policy.toMap(),
        'confirmed_record_ids': confirmedRecordIds
            .map((r) => {'collection': r.collection, 'record_id': r.recordId})
            .toList(),
        if (approvalNote != null) 'approval_note': approvalNote,
      };
}
