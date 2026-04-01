# Debug Inspector

[← Back to Index](README.md)

NodeDB includes a runtime inspection system with 14 panels covering every engine, plus HTTP/WebSocket access, a Flutter UI, and a plugin SDK for custom panels.

## Overview

The inspector provides read-only access to database internals for debugging and monitoring. It consists of four layers:

1. **SDK** (`inspector_sdk`) — Abstract interfaces for panels, registries, and plugins (pure Dart)
2. **Data Layer** (`nodedb_inspector`) — 14 panel classes implementing `InspectorPanel`, with `PanelRegistry` and `CommandDispatcher`
3. **Server** (`nodedb_inspector`) — HTTP + WebSocket server for remote access
4. **Flutter UI** (`nodedb_inspector_flutter`) — Registry-driven NavigationRail panel viewer with `PanelWidgetRegistry`

## Setup

### Data Layer Only

```dart
import 'package:nodedb_inspector/nodedb_inspector.dart';

final inspector = NodeDbInspector(
  db,
  config: InspectorConfig(
    port: 8110,
    passcode: 'optional-auth',
    cacheTtl: Duration(seconds: 5),
  ),
);

// Get a full database snapshot
final snapshot = inspector.snapshot();
print(snapshot['nosql']); // {collections: {public.users: 5}, totalDocuments: 5}
```

### With HTTP/WebSocket Server

```dart
await inspector.start(); // Starts server on configured port

// HTTP endpoints:
// GET /api/snapshot — full database summary (JSON)
// GET /api/command/<panel>/<action> — execute panel command

// WebSocket:
// ws://localhost:8110/ws — real-time updates every cacheTtl interval
```

### Flutter UI Integration

```dart
import 'package:nodedb_inspector_flutter/nodedb_inspector_flutter.dart';

// As a screen
InspectorScreen(databases: [db1, db2])

// As an overlay
NodeInspectorOverlay(
  databases: [db1, db2],
  child: MyApp(),
)
```

## 14 Panels

### 1. NoSQL Panel

Collection statistics, document counts, samples:

```dart
inspector.nosql.summary();
// {collections: {'public.users': 5, 'public.products': 12}, totalDocuments: 17}
```

### 2. Schema Panel

Schema metadata, field definitions, indexes:

```dart
inspector.schema.summary();
// {schemas: ['public', 'analytics'], fingerprint: 'abc123...', entries: [...]}
```

### 3. Graph Panel

Node/edge counts, algorithm results:

```dart
inspector.graph?.summary();
// {nodeCount: 50, edgeCount: 120, labels: ['Person', 'City']}
```

### 4. Vector Panel

Index statistics, collection configs:

```dart
inspector.vector?.summary();
// {dimension: 128, metric: 'cosine', recordCount: 1000}
```

### 5. Federation Panel

Peer and group listings:

```dart
inspector.federation.summary();
// {peerCount: 3, groupCount: 1, activePeers: 2, peers: [...]}
```

### 6. DAC Panel

Access control rules by collection/permission:

```dart
inspector.dac?.summary();
// {ruleCount: 8, byCollection: {'public.users': 3}, byPermission: {'allow': 5, 'deny': 2, 'redact': 1}}
```

### 7. Provenance Panel

Envelope statistics, confidence distribution:

```dart
inspector.provenance?.summary();
// {envelopeCount: 150, byStatus: {'verified': 80, 'unverified': 70}, avgConfidence: 0.78}
```

### 8. Key Resolver Panel

Public key registry status:

```dart
inspector.keyResolver?.summary();
// {keyCount: 5, byTrustLevel: {'explicit': 4, 'revoked': 1}}
```

### 9. Trigger Panel

Registered triggers and their status:

```dart
inspector.triggers.summary();
// {triggerCount: 3, enabled: 2, disabled: 1, triggers: [...]}
```

### 10. Singleton Panel

Singleton collections and current values:

```dart
inspector.singletons.summary();
// {singletonCount: 2, singletons: [{collection: 'app_config', data: {...}}]}
```

### 11. Preference Panel

Preference stores and key counts:

```dart
inspector.preferences.summary();
// {storeCount: 1, stores: [{name: 'user_prefs', keyCount: 3, keys: ['theme', ...]}]}
```

### 12. Access History Panel

Access tracking statistics:

```dart
inspector.accessHistory.summary();
// {totalEvents: 500, oldestEvent: '...', newestEvent: '...'}
```

### 13. AI Panel

AI adapter status and statistics:

```dart
inspector.ai?.summary();
// {aiQueryEnabled: true, aiProvenanceEnabled: true, enabledCollections: [...]}
```

## Inspector Config

```dart
class InspectorConfig {
  final int port;              // HTTP/WS server port (default: 8110)
  final String? passcode;      // Optional authentication
  final Duration cacheTtl;     // Cache refresh interval (default: 5s)
}
```

## Plugin System

The `inspector_sdk` package provides abstract interfaces for extending the inspector with custom panels.

### Custom Panel

```dart
import 'package:inspector_sdk/inspector_sdk.dart';

class MyCustomPanel implements InspectorPanel {
  @override
  PanelDescriptor get descriptor => const PanelDescriptor(
    id: 'myPanel',
    displayName: 'My Panel',
    description: 'Custom inspector panel',
    iconHint: 'extension',
    sortOrder: 100,
    category: 'custom',
    actions: [
      PanelAction(name: 'status'),
    ],
  );

  @override
  bool get isAvailable => true;

  @override
  Map<String, dynamic> summary() => {'status': 'ok'};

  @override
  dynamic dispatch(String action, Map<String, dynamic> params) {
    switch (action) {
      case 'status': return summary();
      default: throw ArgumentError('Unknown action: $action');
    }
  }
}
```

### Register via Plugin

```dart
class MyPlugin implements InspectorPlugin {
  @override
  String get id => 'my-plugin';

  @override
  void register(PanelRegistry panels, DataSourceRegistry sources) {
    panels.register(MyCustomPanel());
  }

  @override
  void unregister(PanelRegistry panels, DataSourceRegistry sources) {
    panels.unregister('myPanel');
  }
}

inspector.registerPlugin(MyPlugin());
```

### Custom Flutter Widget

```dart
final registry = PanelWidgetRegistry();
registry.register('myPanel', (panel) => MyPanelWidget(panel: panel));

InspectorScreen(
  inspector: inspector,
  widgetRegistry: registry,
)
```

### PanelRegistry

All 14 built-in panels are registered in `PanelRegistry` at startup, sorted by `sortOrder`. The registry provides:

- `available` — panels where `isAvailable` is true, sorted by sortOrder
- `get(id)` / `getAs<T>(id)` — lookup by panel ID with typed cast
- `enabledPanelIds()` — IDs of available panels
- Listener notifications on register/unregister

### CommandDispatcher

Replaces the hardcoded command router. Dispatches commands to panels via the registry:

- `dispatch('panel.action', params)` — routes to `panel.dispatch(action, params)`
- `dispatch('snapshot', params)` — invokes custom registered command
- `panelDescriptors()` — metadata for all available panels

## Flutter UI

The `nodedb_inspector_flutter` package provides a dark-themed panel viewer:

- **NavigationRail** — left sidebar with icons from panel `iconHint` strings
- **Registry-driven** — panels discovered via `PanelRegistry.available`
- **Extensible** — custom panel widgets via `PanelWidgetRegistry`
- **Dark theme** — consistent Material 3 dark styling
- **Responsive** — adapts to screen size
- **Real-time** — auto-refreshes at cacheTtl interval

### Reusable Widgets

- Status chips (connected/disconnected/warning)
- Document preview cards
- Trigger list with enable/disable
- Provenance confidence gauge
- Key trust level badges

## Dart Packages

**`inspector_sdk`** — abstract plugin interfaces (pure Dart):
- `InspectorPanel` — panel interface with descriptor, dispatch, summary
- `PanelDescriptor` / `PanelAction` — panel metadata
- `PanelRegistry` / `DataSourceRegistry` — typed registries
- `InspectorPlugin` — bundled panel + data source registration
- `CommandDispatcher` — registry-driven command routing

**`nodedb_inspector`** — data extraction + server:
- `NodeDbInspector` — facade with `PanelRegistry` and `registerPlugin()`
- 14 `*Panel` classes implementing `InspectorPanel`
- `InspectorServer` — HTTP + WebSocket
- `CommandRouter` — delegates to `CommandDispatcher`
- `JsonSerializer` — response formatting

**`nodedb_inspector_flutter`** — Flutter UI:
- `InspectorScreen` — registry-driven screen with optional `PanelWidgetRegistry`
- `NodeInspectorOverlay` — floating debug button overlay
- `PanelWidgetRegistry` — maps panel IDs to widget builders
- 14 panel views (one per engine)
- 5 reusable helper widgets

## Related Pages

- [Architecture](architecture.md) — how inspector fits in the system
- [Dart Packages](dart-packages.md) — inspector package details
- [Getting Started](getting-started.md) — enabling the inspector
