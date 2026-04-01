# nodedb_inspector_flutter

Flutter debug inspector overlay widget for NodeDB databases. Wraps your app with a floating debug button that opens a full-featured inspector screen with 14 panels covering every NodeDB engine.

Part of the [NodeDB](https://github.com/mshakib/nodedb) monorepo.

## Installation

```yaml
dependencies:
  nodedb_inspector_flutter:
    path: ../nodedb_inspector_flutter
```

## Usage

Wrap your app root with `NodeInspectorOverlay`:

```dart
import 'package:nodedb_inspector_flutter/nodedb_inspector_flutter.dart';

runApp(
  NodeInspectorOverlay(
    db: myNodeDB,
    enabled: kDebugMode,
    child: MyApp(),
  ),
);
```

Tap the floating bug icon to open the inspector. The inspector provides panels for Dashboard, NoSQL, Graph, Vector, Federation, DAC, Provenance, Keys, Schema, Triggers, Singletons, Preferences, Access History, and AI.
