/// Configuration for the transport engine (WebSocket + TLS networking).
///
/// Controls how this database communicates with peers over the network.
/// Pass to [NodeDB.open] via the `transportConfig` parameter.
class TransportConfig {
  /// Address to listen on for incoming peer connections.
  ///
  /// Format: `"host:port"` (e.g. `"0.0.0.0:9400"`).
  /// Use `"0.0.0.0"` to listen on all interfaces.
  final String listenAddr;

  /// Whether to enable mDNS auto-discovery of peers on the local network.
  ///
  /// When enabled, devices on the same WiFi will find each other
  /// automatically (~30 seconds). Disable if using manual/QR pairing only.
  final bool mdnsEnabled;

  /// Bootstrap peers to connect to on startup.
  ///
  /// Format: `["wss://host:port", ...]`.
  /// Useful for connecting to known peers without mDNS.
  final List<String> seedPeers;

  /// Query routing policy for federated queries.
  ///
  /// - `localOnly` — never query peers
  /// - `queryPeersOnMiss` — query peers when local results are empty
  /// - `queryPeersAlways` — always query peers in addition to local
  /// - `queryPeersExplicitly` — only query peers when explicitly requested
  final QueryPolicy queryPolicy;

  /// How often to broadcast the peer list (in seconds).
  final int gossipIntervalSeconds;

  /// Number of peers to send gossip messages to each round.
  final int gossipFanOut;

  /// Maximum hops a gossip message can travel.
  final int gossipTtl;

  /// Optional storage path for transport audit logs.
  final String? storagePath;

  /// Optional Ed25519 identity key (64-character hex string).
  ///
  /// If not provided, a new identity is generated on each open.
  /// Provide a stable key to maintain the same peer ID across restarts.
  final String? identityKeyHex;

  /// Trusted peer public keys (64-character hex Ed25519 public keys).
  ///
  /// If non-empty, only peers whose public key matches one in this list
  /// will be accepted during the handshake. Public keys can be exchanged
  /// via QR code. If empty (default), all peers are accepted.
  final List<String> trustedPeerKeys;

  /// Whether to require user-approved pairing before accepting peers.
  ///
  /// When enabled, unknown peers trigger a pairing request that must be
  /// approved via [TransportEngine.approvePairing]. Previously paired
  /// devices reconnect automatically without re-pairing.
  final bool requirePairing;

  /// Globally unique user ID (UUID) exchanged during pairing.
  ///
  /// Stored in the pairing record and verified on reconnect.
  final String? userId;

  /// Human-readable device name exchanged during pairing.
  final String? deviceName;

  const TransportConfig({
    this.listenAddr = '0.0.0.0:9400',
    this.mdnsEnabled = true,
    this.seedPeers = const [],
    this.queryPolicy = QueryPolicy.queryPeersOnMiss,
    this.gossipIntervalSeconds = 30,
    this.gossipFanOut = 3,
    this.gossipTtl = 5,
    this.storagePath,
    this.identityKeyHex,
    this.trustedPeerKeys = const [],
    this.requirePairing = false,
    this.userId,
    this.deviceName,
  });

  /// Convert to the map format expected by the FFI layer.
  Map<String, dynamic> toMap() => {
        'listen_addr': listenAddr,
        'mdns_enabled': mdnsEnabled,
        if (seedPeers.isNotEmpty) 'seed_peers': seedPeers,
        'query_policy': queryPolicy.toFfiString(),
        'gossip_interval_seconds': gossipIntervalSeconds,
        'gossip_fan_out': gossipFanOut,
        'gossip_ttl': gossipTtl,
        if (storagePath != null) 'path': storagePath,
        if (identityKeyHex != null) 'identity_key': identityKeyHex,
        if (trustedPeerKeys.isNotEmpty) 'trusted_peer_keys': trustedPeerKeys,
        if (requirePairing) 'require_pairing': true,
        if (userId != null) 'user_id': userId,
        if (deviceName != null) 'device_name': deviceName,
      };
}

/// Query routing policy for federated peer queries.
enum QueryPolicy {
  /// Never query peers — local results only.
  localOnly,

  /// Query peers when local results are empty.
  queryPeersOnMiss,

  /// Always query peers in addition to local.
  queryPeersAlways,

  /// Only query peers when explicitly requested via [findAllFederated].
  queryPeersExplicitly;

  String toFfiString() => switch (this) {
        localOnly => 'local_only',
        queryPeersOnMiss => 'query_peers_on_miss',
        queryPeersAlways => 'query_peers_always',
        queryPeersExplicitly => 'query_peers_explicitly',
      };
}
