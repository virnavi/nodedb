/// Configuration for the NodeDB Debug Inspector.
class InspectorConfig {
  /// WebSocket port for remote inspector connections.
  final int port;

  /// Optional passcode for inspector access protection.
  final String? passcode;

  /// Time-to-live for cached statistics.
  final Duration cacheTtl;

  const InspectorConfig({
    this.port = 8110,
    this.passcode,
    this.cacheTtl = const Duration(seconds: 5),
  });
}
