import 'package:inspector_sdk/inspector_sdk.dart';

import '../inspector.dart';

/// Routes WebSocket commands to inspector panel methods via [CommandDispatcher].
class CommandRouter {
  final CommandDispatcher _dispatcher;

  CommandRouter(NodeDbInspector inspector)
      : _dispatcher = CommandDispatcher(inspector.panelRegistry) {
    _dispatcher.registerCommand('snapshot', (_) => inspector.snapshot());
  }

  /// Dispatch a command and return a JSON-serializable result.
  dynamic dispatch(String cmd, Map<String, dynamic> params) {
    return _dispatcher.dispatch(cmd, params);
  }
}
