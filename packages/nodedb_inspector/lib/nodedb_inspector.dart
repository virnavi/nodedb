/// Debug inspector data layer for NodeDB databases.
///
/// Provides structured panel-based access to all engine data
/// for debugging UIs (Flutter overlay, web dashboard, CLI).
library nodedb_inspector;

// Re-export inspector SDK for plugin authors
export 'package:inspector_sdk/inspector_sdk.dart';

export 'src/inspector.dart';
export 'src/inspector_config.dart';
export 'src/panels/nosql_panel.dart';
export 'src/panels/graph_panel.dart';
export 'src/panels/vector_panel.dart';
export 'src/panels/federation_panel.dart';
export 'src/panels/dac_panel.dart';
export 'src/panels/provenance_panel.dart';
export 'src/panels/keyresolver_panel.dart';
export 'src/panels/schema_panel.dart';
export 'src/panels/trigger_panel.dart';
export 'src/panels/singleton_panel.dart';
export 'src/panels/preference_panel.dart';
export 'src/panels/access_history_panel.dart';
export 'src/panels/ai_panel.dart';
export 'src/server/inspector_server.dart';
export 'src/server/command_router.dart';
export 'src/server/json_serializers.dart';
