import '../panel/panel_result.dart';
import '../registry/panel_registry.dart';

/// Registry-driven command dispatcher.
///
/// Routes commands to panels via the [PanelRegistry]. The command protocol:
/// - `{cmd: 'panel', panel: '<id>', action: '<action>', ...}`
/// - `{cmd: 'enabledPanels'}`
/// - `{cmd: 'panelDescriptors'}`
/// - Custom commands registered via [registerCommand].
class CommandDispatcher {
  final PanelRegistry _registry;
  final Map<String, dynamic Function(Map<String, dynamic>)> _customCommands =
      {};

  CommandDispatcher(this._registry);

  /// Register a custom top-level command.
  void registerCommand(
    String cmd,
    dynamic Function(Map<String, dynamic>) handler,
  ) {
    _customCommands[cmd] = handler;
  }

  /// Dispatch a command. Returns a JSON-serializable result.
  dynamic dispatch(String cmd, Map<String, dynamic> params) {
    switch (cmd) {
      case 'enabledPanels':
        return _registry.enabledPanelIds();

      case 'panelDescriptors':
        return _registry.descriptors;

      case 'panel':
        return _dispatchPanel(params);

      default:
        final custom = _customCommands[cmd];
        if (custom != null) return custom(params);
        throw ArgumentError('Unknown command: $cmd');
    }
  }

  dynamic _dispatchPanel(Map<String, dynamic> params) {
    final panelId = params['panel'] as String?;
    final action = params['action'] as String?;
    if (panelId == null || action == null) {
      throw ArgumentError('panel and action are required');
    }

    final panel = _registry.get(panelId);
    if (panel == null) {
      throw ArgumentError('Unknown panel: $panelId');
    }

    if (!panel.isAvailable) {
      return PanelDisabled(panelId).toJson();
    }

    return panel.dispatch(action, params);
  }
}
