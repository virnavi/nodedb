# Dart Packages Reference

[← Back to Index](README.md)

NodeDB's Dart packages are under `packages/`. They provide typed wrappers, code generation, testing utilities, and Flutter integration.

## Package Dependency Graph

```
nodedb_ffi (lowest — raw dart:ffi)
    ↓
nodedb (core — facade, engines, models, annotations)
    ↓
├── nodedb_generator (build_runner code gen)
├── nodedb_test (test utilities)
├── inspector_sdk (pure Dart — panel/registry/plugin interfaces)
│       ↓
│   nodedb_inspector (panels implement InspectorPanel + server)
│       ↓
│   nodedb_inspector_flutter (PanelWidgetRegistry + Flutter UI)
│
├── nodedb_flutter_libs (pre-compiled native binaries)
│
└── nodedb_example (example Flutter app)
```

## Package Reference

### nodedb_ffi

**Purpose**: Raw `dart:ffi` bindings to `libnodedb_ffi`.

**Key Exports**:

| Class/Function | Description |
|----------------|-------------|
| `NodeDbBindings` | Holds all FFI function pointers (35+) |
| `openRaw()` | Open an engine, return handle |
| `executeRaw()` | Execute a request, return response bytes |
| `NodeDbFfiException` | FFI-level error (code + message) |

**How it works**: Loads the dynamic library, looks up C symbols, provides typed Dart wrappers for each FFI function.

---

### nodedb

**Purpose**: Core library — the NodeDB facade, 10 typed engine wrappers, models, annotations, and query builders.

**Key Exports (60+)**:

#### Facade

| Class | Description |
|-------|-------------|
| `NodeDB` | Main entry point — `open()`, delegates to all engines |

#### Engines (10)

| Class | Description |
|-------|-------------|
| `NoSqlEngine` | Document CRUD, schemas, triggers, preferences, trim |
| `GraphEngine` | Node/edge CRUD, algorithms |
| `VectorEngine` | Vector insert/search/delete |
| `FederationEngine` | Peer/group management |
| `DacEngine` | Access control rules |
| `TransportEngine` | WebSocket networking, mesh, pairing |
| `ProvenanceEngine` | Data lineage tracking |
| `KeyResolverEngine` | Public key registry |
| `AiProvenanceEngine` | AI confidence assessment |
| `AiQueryEngine` | AI query processing |

#### Models

| Class | Description |
|-------|-------------|
| `Document` | NoSQL document (id, collection, data, timestamps) |
| `WriteOp` | Atomic write operation |
| `GraphNode` / `GraphEdge` | Graph primitives |
| `VectorRecord` / `SearchResult` | Vector types |
| `NodePeer` / `NodeGroup` | Federation types |
| `ProvenanceEnvelope` | 28-field provenance metadata |
| `KeyEntry` | Public key entry |
| `TransportConfig` | Network configuration |
| `MeshConfig` | Mesh networking configuration |
| `NodeDbSchema` / `SchemaField` | Schema metadata |
| `TrimPolicy` / `TrimReport` | Trim configuration and results |

#### Query

| Class | Description |
|-------|-------------|
| `FilterQuery<T>` | Fluent query builder |
| `WithProvenance<T>` | Document + provenance pair |
| `FederatedResult<T>` | Document + source peer |

#### Annotations (15+)

| Annotation | Target | Description |
|-----------|--------|-------------|
| `@collection` | Class | NoSQL collection |
| `@node` | Class | Graph node |
| `@Edge(from, to)` | Class | Graph edge |
| `@embedded` | Class | Nested object |
| `@preferences` | Class | Typed preference store |
| `@Index` | Field | Index configuration |
| `@VectorField` | Field | Vector embedding |
| `@Enumerated` | Field | Enum as string |
| `@Trimmable` | Class | Enable auto-trim |
| `@neverTrim` | Class | Prevent trimming |
| `@Trigger` | Class | Database trigger |
| `@ProvenanceConfig` | Class | Provenance settings |
| `@Access` | Field | Access control |
| `@Shareable` | Class | Federation sharing |
| `@noDao` | Class | Skip DAO generation |

#### Adapters

| Class | Description |
|-------|-------------|
| `AiQueryAdapter` | Abstract — implement for AI query fallback |
| `AiProvenanceAdapter` | Abstract — implement for AI provenance assessment |

#### Errors

| Class | Description |
|-------|-------------|
| `NodeDbException` | Base exception (code + message) |
| `StorageException` | Sled storage error |
| `NotFoundException` | Record not found |
| `TransportException` | Network error |
| ... 15+ subclasses | One per error category |

---

### nodedb_generator

**Purpose**: `build_runner` code generation from annotations.

**Generators**:

| Generator | Input | Output |
|-----------|-------|--------|
| `CollectionGenerator` | `@collection` class | Schema, DAO, filters, serialization |
| `NodeGenerator` | `@node` class | Schema, DAO, filters, serialization |
| `EdgeGenerator` | `@Edge` class | Schema, DAO, filters, serialization |
| `PreferencesAnnotationGenerator` | `@preferences` class | Typed preference accessors |
| `DaoRegistryGenerator` | All DAOs | Central DAO factory |

**Generated per `@collection`**:
- `*Schema` — const metadata
- `*DaoBase` — abstract DAO with CRUD methods
- `*FilterExtension` — typed filter/sort methods on `FilterQuery<T>`
- `_$*FromMap()` / `_$*ToMap()` — serialization
- Provenance support (if `@ProvenanceConfig`)
- Trim methods (if `@Trimmable`)
- Trigger registration (if `@Trigger`)

**Usage**:
```bash
dart run build_runner build
```

---

### nodedb_flutter_libs

**Purpose**: Flutter plugin packaging pre-compiled native libraries.

**Platforms**: Android, iOS, macOS, Linux, Windows

**Configuration**: `ffiPlugin: true` in `pubspec.yaml` — Flutter auto-loads the correct native library.

---

### nodedb_test

**Purpose**: Test utilities for writing NodeDB tests.

**Key Exports**:

| Class/Function | Description |
|----------------|-------------|
| `TestNodeDB.create()` | Create temp database with optional engines |
| `TestNodeDB.cleanUp()` | Delete all temp directories |
| `hasDocumentId(int)` | Matcher for document ID |
| `hasDocumentData(Map)` | Matcher for document data |
| `isDocument(id?, data?)` | Composite document matcher |
| `hasDocumentCount(int)` | List length matcher |
| `hasNodeLabel(String)` | Graph node label matcher |
| `connectsNodes(int, int)` | Edge endpoint matcher |

---

### inspector_sdk

**Purpose**: Abstract interfaces for the inspector plugin system. Pure Dart — no Flutter dependency.

**Key Exports**:

| Class | Description |
|-------|-------------|
| `InspectorPanel` | Abstract panel interface (descriptor, dispatch, summary) |
| `PanelDescriptor` | Panel metadata (id, displayName, iconHint, sortOrder, category, actions) |
| `PanelAction` / `PanelActionParam` | Action descriptors with typed parameters |
| `PanelResult` | Sealed result type (PanelSuccess, PanelError, PanelDisabled) |
| `PanelRegistry` | Register/lookup/iterate panels, sorted by sortOrder |
| `InspectorDataSource` | Abstract data source interface |
| `DataSourceDescriptor` | Data source metadata |
| `DataSourceRegistry` | Register/lookup data sources |
| `InspectorPlugin` | Bundled registration of panels + data sources |
| `CommandDispatcher` | Registry-driven command routing with custom commands |
| `CommandContext` | Command parameters + client ID |
| `JsonSerializable` | Mixin for `toJson()` |
| `SerializerRegistry` | Type-keyed serializer lookup |

---

### nodedb_inspector

**Purpose**: Debug data extraction layer with 14 panels implementing `InspectorPanel` + HTTP/WebSocket server.

**Key Exports** (re-exports `inspector_sdk`):

| Class | Description |
|-------|-------------|
| `NodeDbInspector` | Facade with `PanelRegistry`, `registerPlugin()` |
| `InspectorConfig` | Port, passcode, cache TTL |
| `NoSqlPanel` | Collection stats (implements `InspectorPanel`) |
| `GraphPanel` | Node/edge stats, algorithms |
| `VectorPanel` | Index stats |
| `FederationPanel` | Peer/group info |
| `DacPanel` | Rule counts |
| `ProvenancePanel` | Envelope stats |
| `KeyResolverPanel` | Key counts |
| `SchemaPanel` | Schema metadata |
| `TriggerPanel` | Trigger info |
| `SingletonPanel` | Singleton data |
| `PreferencePanel` | Preference stores |
| `AccessHistoryPanel` | History stats |
| `AiPanel` | AI adapter status |
| `InspectorServer` | HTTP + WebSocket server |
| `CommandRouter` | Delegates to `CommandDispatcher` |

---

### nodedb_inspector_flutter

**Purpose**: Flutter UI for the debug inspector with plugin support.

**Key Exports**:

| Widget/Class | Description |
|--------------|-------------|
| `InspectorScreen` | Registry-driven screen with NavigationRail |
| `NodeInspectorOverlay` | Floating debug button overlay |
| `PanelWidgetRegistry` | Maps panel IDs to widget builder functions |

Features:
- Registry-driven panel discovery via `PanelRegistry.available`
- Extensible via `PanelWidgetRegistry` for custom panel widgets
- `iconHint` string → `IconData` mapping for Flutter-free SDK
- 14 built-in panel views (one per engine)
- Dark theme (Material 3)
- Responsive layout
- 5 reusable helper widgets

---

### nodedb_example

**Purpose**: Example Flutter app demonstrating all features.

**Structure**:

| Directory | Contents |
|-----------|----------|
| `lib/models/` | 5 annotated model classes (User, UserPrefs, Product, Category, Order) |
| `lib/database/` | UserDatabase, ProductDatabase, MeshService |
| `lib/screens/` | 5 screens (Users, Products, Orders, Mesh, Inspector) |
| `lib/generated/` | 2340+ lines of generated code |

**Demonstrates**:
- `@collection` with String IDs and UUID v7
- `@preferences` with typed accessors
- `@Index(unique: true)` for email
- Multi-database mesh (users DB + products DB)
- Federated product search across devices
- QR code pairing
- Inspector overlay integration
- Provenance tracking

## Related Pages

- [Rust Crates](rust-crates.md) — Rust-side implementation
- [FFI Protocol](ffi.md) — MessagePack protocol details
- [Code Generation](code-generation.md) — annotation reference
- [Getting Started](getting-started.md) — setup guide
