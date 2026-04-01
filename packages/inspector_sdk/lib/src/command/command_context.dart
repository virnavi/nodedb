/// Context available during command dispatch.
class CommandContext {
  /// The raw parameters from the command message.
  final Map<String, dynamic> params;

  /// Optional requesting client identifier.
  final String? clientId;

  const CommandContext({
    required this.params,
    this.clientId,
  });
}
