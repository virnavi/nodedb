/// Structured result from a panel action dispatch.
sealed class PanelResult {
  const PanelResult();
}

/// Successful result containing JSON-serializable data.
class PanelSuccess extends PanelResult {
  final dynamic data;
  const PanelSuccess(this.data);
}

/// Error result from a panel action.
class PanelError extends PanelResult {
  final String code;
  final String message;
  const PanelError(this.code, this.message);

  Map<String, dynamic> toJson() => {'error': code, 'message': message};
}

/// Panel is disabled / not available.
class PanelDisabled extends PanelResult {
  final String panelId;
  const PanelDisabled(this.panelId);

  Map<String, dynamic> toJson() =>
      {'error': 'panel_disabled', 'panel': panelId};
}
