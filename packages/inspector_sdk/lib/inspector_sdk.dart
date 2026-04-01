/// Plugin/extension SDK for NodeDB inspector panels and data sources.
///
/// Provides abstract interfaces, registries, and command protocol integration
/// for building custom inspector panels and data sources.
library inspector_sdk;

// Panel contracts
export 'src/panel/inspector_panel.dart';
export 'src/panel/panel_action.dart';
export 'src/panel/panel_descriptor.dart';
export 'src/panel/panel_result.dart';

// Data source contracts
export 'src/data_source/inspector_data_source.dart';
export 'src/data_source/data_source_descriptor.dart';

// Registry
export 'src/registry/panel_registry.dart';
export 'src/registry/data_source_registry.dart';
export 'src/registry/inspector_plugin.dart';

// Command protocol
export 'src/command/command_context.dart';
export 'src/command/command_dispatcher.dart';

// Serialization
export 'src/serialization/json_serializable.dart';
export 'src/serialization/serializer_registry.dart';
