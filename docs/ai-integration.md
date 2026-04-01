# AI Integration

[← Back to Index](README.md)

NodeDB provides pluggable AI adapters for query augmentation and provenance assessment. AI functionality is opt-in and requires user-provided adapter implementations.

## Architecture

```
┌──────────────────────┐     ┌──────────────────────┐
│  Your AI Adapter     │     │  Your AI Adapter      │
│  (implements         │     │  (implements           │
│   AiQueryAdapter)    │     │   AiProvenanceAdapter) │
└──────────┬───────────┘     └──────────┬────────────┘
           │                            │
    ┌──────┴───────┐            ┌───────┴──────┐
    │ AiQueryEngine│            │AiProvenance  │
    │ (Rust)       │            │Engine (Rust) │
    └──────┬───────┘            └───────┬──────┘
           │                            │
    ┌──────┴───────┐            ┌───────┴──────┐
    │ NoSQL Engine │            │ Provenance   │
    │              │            │ Engine       │
    └──────────────┘            └──────────────┘
```

## AI Query Engine

### Purpose

When local and federated queries return no results, the AI query adapter can generate synthetic results from external sources (LLMs, APIs, embeddings, etc.).

### Setup

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  provenanceEnabled: true, // Required — AI results get provenance tracking
);

// Configure the adapter
db.configureAiQuery(
  adapter: MyAiQueryAdapter(),
  enabledCollections: ['public.products', 'public.articles'],
  minimumWriteConfidence: 0.80,
);
```

### Adapter Interface

```dart
abstract class AiQueryAdapter {
  /// Called when local + federated results are empty.
  /// Return AI-generated results with confidence scores.
  Future<List<AiQueryResult>> findResults(
    String collection,
    Map<String, dynamic>? filter,
    Map<String, dynamic>? schema,
  );
}
```

### AiQueryResult

```dart
class AiQueryResult {
  final Map<String, dynamic> data;     // Document data
  final double confidence;              // 0.0–1.0
  final String? sourceExplanation;     // How AI produced this
  final String? externalSourceUri;     // External data source URL
  final Map<String, dynamic>? tags;    // AI-generated tags
}
```

### Processing Pipeline

1. AI adapter returns `List<AiQueryResult>`
2. Schema validation (if schema provided): reject invalid results
3. Confidence check: reject results below `minimumWriteConfidence`
4. Accepted results are **written to the local database**
5. Provenance envelope created with:
   - `sourceType: 'ai_query'`
   - `aiOriginated: true`
   - `aiOriginTag: 'ai-query:{collection}:{utc}'`
   - `confidenceFactor`: blended AI + deterministic confidence
6. Results returned to caller

### Usage

```dart
// Automatic — via query pipeline
final results = db.findAllFull(
  'public.products',
  filter: filter,
);
// Tries: local → federation → AI fallback

// Explicit — via query builder flag
final results = productDao.findWhere(
  (q) => q.nameContains('rare-item').withAiQuery(),
);

// Direct — via engine
final written = db.aiQuery!.processResults(
  'public.products',
  aiResults,
  schema: productSchema,
);
```

## AI Provenance Engine

### Purpose

AI-powered assessment of data provenance: confidence scoring, conflict resolution, anomaly detection, and source classification.

### Setup

```dart
db.configureAiProvenance(
  adapter: MyAiProvenanceAdapter(),
  enabledCollections: ['public.sensor_readings'],
  blendWeight: 0.3,  // 30% AI, 70% deterministic
);
```

### Adapter Interface

```dart
abstract class AiProvenanceAdapter {
  /// Assess a provenance envelope and suggest confidence adjustments.
  Future<AiProvenanceAssessment> assessResult(ProvenanceEnvelope envelope);
}
```

### Operations

#### Confidence Assessment

```dart
// Apply AI-suggested confidence
db.aiProvenance!.applyAssessment(
  envelopeId: envelope.id,
  suggestedConfidence: 0.92,
  sourceType: 'user',
  reasoning: 'High-quality user input with consistent metadata',
  tags: {'quality': 'high', 'method': 'llm-assessment'},
);
```

**Blending formula**: `final = deterministic * (1 - weight) + ai * weight`, clamped [0, 1]

**Post-blend boost**: Verified envelopes get +0.10 after blending.

#### Conflict Resolution

```dart
// Resolve conflicting provenance between two records
db.aiProvenance!.applyConflictResolution(
  envelopeIdA: env1.id,
  envelopeIdB: env2.id,
  deltaA: 0.05,       // Confidence adjustment for A
  deltaB: -0.10,      // Confidence adjustment for B
  preference: 'prefer_a',
  reasoning: 'Source A has more recent verification',
);
```

Preferences: `prefer_a`, `prefer_b`, `merge`.

#### Anomaly Detection

```dart
// Flag anomalous records
db.aiProvenance!.applyAnomalyFlags(
  'public.sensor_readings',
  [
    AnomalyFlag(
      recordId: 42,
      severity: 'high',
      description: 'Temperature reading 200°C exceeds sensor range',
      penalty: 0.3,  // Confidence reduction
    ),
  ],
);
```

Severity levels: `low`, `medium`, `high`.

**Penalty formula**: `confidence = confidence * (1 - penalty)`

#### Source Classification

```dart
// Classify source credibility
db.aiProvenance!.applySourceClassification(
  envelopeId: envelope.id,
  sourceType: 'sensor',
  credibilityPrior: 0.8,
  reasoning: 'Calibrated sensor with regular maintenance',
);
```

## Confidence Lifecycle

```
Initial confidence (by source type)
  → Corroboration boost (+10% of remaining)
  → Conflict penalty (*0.85)
  → AI assessment blending
  → Verification boost (+0.10) / failure penalty (*0.5)
  → Anomaly penalty
  → Age decay (half-life)
```

## Enabled Collections

Both AI engines are **collection-scoped** — only process results for explicitly enabled collections:

```dart
// AI Query: only products and articles
db.configureAiQuery(
  enabledCollections: ['public.products', 'public.articles'],
);

// AI Provenance: only sensor readings
db.configureAiProvenance(
  enabledCollections: ['public.sensor_readings'],
);
```

Attempting to process a non-enabled collection returns `ERR_AI_QUERY_COLLECTION_NOT_ENABLED` or `ERR_AI_PROVENANCE_COLLECTION_NOT_ENABLED`.

## Rust Implementation

**Crate**: `nodedb-ai-query` → depends on `nodedb-nosql`, `nodedb-provenance`

Key types:
- `AiQueryEngine` — wraps Database + ProvenanceEngine
- `AiQueryConfig` — min confidence, max results, enabled collections
- `AiQueryResult` — result with confidence and metadata
- `AiQuerySchema` — JSON schema for validation

**Crate**: `nodedb-ai-provenance` → depends on `nodedb-provenance`

Key types:
- `AiProvenanceEngine` — wraps ProvenanceEngine
- `AiProvenanceConfig` — blend weight, enabled collections
- `AiProvenanceAssessment` — suggested confidence with reasoning
- `AiConflictResolution` — delta adjustments for conflicts
- `AiAnomalyFlag` — anomaly with penalty and severity

## Related Pages

- [Data Provenance](provenance.md) — underlying provenance system
- [Query System](query-system.md) — `withAiQuery()` flag, `findAllFull()` pipeline
- [NoSQL Engine](nosql-engine.md) — document storage for AI-written results
