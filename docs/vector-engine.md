# Vector Engine

[← Back to Index](README.md)

The vector engine provides approximate nearest-neighbor (ANN) search using the HNSW (Hierarchical Navigable Small World) algorithm.

## Configuration

Enable with a `VectorOpenConfig` when opening the database:

```dart
final db = NodeDB.open(
  directory: '/path/to/data',
  vectorConfig: VectorOpenConfig(
    dimension: 128,                        // Vector dimensionality
    metric: DistanceMetric.cosine,         // Distance metric
  ),
);
```

### Distance Metrics

| Metric | Description | Range |
|--------|-------------|-------|
| `cosine` | Cosine similarity (1 - cosine_sim) | [0, 2] |
| `euclidean` | L2 distance | [0, ∞) |
| `dotProduct` | Negative dot product | (-∞, ∞) |

## Operations

```dart
final vector = db.vector!;

// Insert a vector with metadata
final record = vector.insert(
  [0.1, 0.2, 0.3, ...], // 128-dimensional vector
  metadata: {'label': 'cat', 'source': 'model-v2'},
);

// Search for k nearest neighbors
final results = vector.search(
  [0.15, 0.22, 0.28, ...], // Query vector
  k: 10,                    // Number of results
  efSearch: 64,              // HNSW search parameter (higher = more accurate)
);

for (final r in results) {
  print('ID: ${r.id}, distance: ${r.distance}, meta: ${r.metadata}');
}

// Update metadata (vector itself is immutable)
vector.updateMetadata(record.id, {'label': 'cat', 'verified': true});

// Delete
vector.delete(record.id);

// Get by ID
final existing = vector.get(record.id);
```

## Data Models

```dart
class VectorRecord {
  final int id;
  final List<double> vector;
  final Map<String, dynamic> metadata;
  final DateTime createdAt;
}

class SearchResult {
  final int id;
  final double distance;
  final Map<String, dynamic> metadata;
}
```

## HNSW Parameters

The HNSW index has configurable hyperparameters:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `dimension` | required | Vector dimensionality |
| `maxElements` | 10000 | Maximum index capacity |
| `m` | 16 | Max connections per node per layer |
| `efConstruction` | 200 | Build-time search parameter |
| `efSearch` | 64 | Query-time search parameter |

Higher `efSearch` gives more accurate results at the cost of speed.

## Annotations

Use `@VectorField` for code generation:

```dart
@collection
class Product {
  String name;
  @VectorField(dimensions: 128, metric: DistanceMetric.cosine)
  List<double> embedding;
}
```

## Rust Implementation

**Crate**: `nodedb-vector` → depends on `nodedb-storage`, `hnsw_rs`

Key types:
- `VectorEngine` — insert, search, delete, metadata management
- `VectorRecord` — stored vector with metadata
- `SearchResult` — query result with distance
- `DistanceMetric` — cosine, euclidean, dot product
- `CollectionConfig` — HNSW hyperparameters

## Related Pages

- [NoSQL Engine](nosql-engine.md) — document storage (metadata can reference vector IDs)
- [AI Integration](ai-integration.md) — AI-powered similarity search
- [Getting Started](getting-started.md) — enabling the vector engine
