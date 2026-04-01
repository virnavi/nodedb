# nodedb_inspector

Debug inspector data layer for NodeDB databases. Provides panel classes for every engine (NoSQL, Graph, Vector, Federation, DAC, Provenance, Key Resolver, Schema, Trigger, Singleton, Preference, Access History, AI) plus a `NodeDbInspector` facade and HTTP/WebSocket server for the web dashboard.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dependencies:
  nodedb_inspector:
    path: ../nodedb_inspector
```

## Usage

```dart
import 'package:nodedb_inspector/nodedb_inspector.dart';

final inspector = NodeDbInspector(db);

// Get a snapshot of all engine states
final snapshot = inspector.snapshot();

// Access individual panels
final nosqlPanel = inspector.nosql;
final collections = nosqlPanel.listCollections();
```

For the Flutter overlay widget, see `nodedb_inspector_flutter`.
