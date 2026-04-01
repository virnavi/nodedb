import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_example/database/mesh_service.dart';
import 'package:qr_flutter/qr_flutter.dart';

/// Mesh screen — shows mesh config, QR pairing, and peer info.
class MeshScreen extends StatefulWidget {
  final MeshService mesh;
  const MeshScreen({super.key, required this.mesh});

  @override
  State<MeshScreen> createState() => _MeshScreenState();
}

class _MeshScreenState extends State<MeshScreen> {
  String? _localIp;

  @override
  void initState() {
    super.initState();
    _resolveLocalIp();
  }

  Future<void> _resolveLocalIp() async {
    try {
      final interfaces = await NetworkInterface.list(
        type: InternetAddressType.IPv4,
        includeLoopback: false,
      );
      for (final iface in interfaces) {
        for (final addr in iface.addresses) {
          if (!addr.isLoopback) {
            setState(() => _localIp = addr.address);
            return;
          }
        }
      }
    } on Exception {
      // Ignore — IP will show as unavailable
    }
  }

  /// Get this peer's public key hex from transport identity.
  String? get _publicKeyHex {
    try {
      final identity = widget.mesh.userDb.db.transport?.identity();
      final keyBytes = identity?['public_key_bytes'];
      if (keyBytes is List) {
        return keyBytes
            .map((b) => (b as int).toRadixString(16).padLeft(2, '0'))
            .join();
      }
    } on Exception {
      // Ignore — key unavailable
    }
    return null;
  }

  /// JSON payload encoded in the QR code.
  String get _qrPayload {
    final pubKey = _publicKeyHex;
    return jsonEncode({
      'nodedb': true,
      'ip': _localIp ?? '?.?.?.?',
      'users_port': 9400,
      'products_port': 9401,
      'mesh': 'nodedb-example',
      if (pubKey != null) 'public_key': pubKey, // ignore: use_null_aware_elements
    });
  }

  bool get _hasTransport =>
      widget.mesh.userDb.db.transport != null ||
      widget.mesh.productDb.db.transport != null;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        // Quick-pair card with QR
        _QuickPairCard(
          localIp: _localIp,
          qrPayload: _qrPayload,
          hasTransport: _hasTransport,
          theme: theme,
          onScanResult: _handleScanResult,
        ),
        const SizedBox(height: 16),

        // How it works
        _HowToConnectCard(theme: theme),
        const SizedBox(height: 16),

        // Pending Pairings
        if (_hasTransport)
          _PendingPairingsCard(
            transport: widget.mesh.userDb.db.transport!,
            theme: theme,
            onStateChanged: () => setState(() {}),
          ),
        if (_hasTransport) const SizedBox(height: 16),

        // Paired Devices
        if (_hasTransport)
          _PairedDevicesCard(
            transport: widget.mesh.userDb.db.transport!,
            theme: theme,
            onStateChanged: () => setState(() {}),
          ),
        if (_hasTransport) const SizedBox(height: 16),

        // Users DB status
        _DatabaseCard(
          title: 'Users Database',
          icon: Icons.people,
          port: '9400',
          db: widget.mesh.userDb.db,
          stats: {'Users': widget.mesh.userDb.users.count().toString()},
          theme: theme,
        ),
        const SizedBox(height: 12),

        // Products DB status
        _DatabaseCard(
          title: 'Products Database',
          icon: Icons.inventory,
          port: '9401',
          db: widget.mesh.productDb.db,
          stats: {
            'Products': widget.mesh.productDb.products.count().toString(),
            'Categories':
                widget.mesh.productDb.categories.count().toString(),
            'Orders': widget.mesh.productDb.orders.count().toString(),
          },
          theme: theme,
        ),
        const SizedBox(height: 16),

        // Architecture
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Architecture', style: theme.textTheme.titleMedium),
                const SizedBox(height: 12),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Text(
                    '  Device A                    Device B\n'
                    '  +--------------+           +--------------+\n'
                    '  | Users DB     |<--mesh--->| Users DB     |\n'
                    '  |  :9400       |   WiFi    |  :9400       |\n'
                    '  | Products DB  |<--mesh--->| Products DB  |\n'
                    '  |  :9401       |   mDNS    |  :9401       |\n'
                    '  +--------------+           +--------------+\n'
                    '       meshName: "nodedb-example"',
                    style: TextStyle(fontFamily: 'monospace', fontSize: 11),
                  ),
                ),
                const SizedBox(height: 8),
                const Text(
                  'Each database uses WebSocket + TLS transport with mDNS '
                  'auto-discovery. Devices on the same WiFi find each other '
                  'automatically. QR codes provide instant manual pairing.',
                  style: TextStyle(fontSize: 13),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  void _handleScanResult(String rawData) {
    try {
      final data = jsonDecode(rawData) as Map<String, dynamic>;
      if (data['nodedb'] != true) {
        _showSnack('Not a NodeDB QR code');
        return;
      }

      final ip = data['ip'] as String;
      final usersPort = data['users_port'] ?? 9400;
      final productsPort = data['products_port'] ?? 9401;
      final peerPublicKey = data['public_key'] as String?;

      if (!_hasTransport) {
        final keyInfo = peerPublicKey != null
            ? '\nPeer key: ${peerPublicKey.substring(0, 12)}...'
            : '';
        _showSnack(
          'Scanned $ip — transport not enabled.$keyInfo',
        );
        return;
      }

      var connected = 0;
      var failed = 0;

      try {
        widget.mesh.userDb.db.transport?.connect('wss://$ip:$usersPort');
        connected++;
      } on Exception {
        failed++;
      }

      try {
        widget.mesh.productDb.db.transport
            ?.connect('wss://$ip:$productsPort');
        connected++;
      } on Exception {
        failed++;
      }

      final keyInfo = peerPublicKey != null
          ? ' key:${peerPublicKey.substring(0, 8)}…'
          : '';

      if (failed == 0) {
        _showSnack('Connected to $ip ($connected dbs)$keyInfo');
      } else {
        _showSnack('Partial: $connected connected, $failed failed');
      }
    } on FormatException {
      _showSnack('Invalid QR code data');
    }
  }

  void _showSnack(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }
}

// ─────────────────────────────────────────────────────────────────
// Quick-Pair Card (QR Generate + Scan)
// ─────────────────────────────────────────────────────────────────

class _QuickPairCard extends StatelessWidget {
  final String? localIp;
  final String qrPayload;
  final bool hasTransport;
  final ThemeData theme;
  final void Function(String data) onScanResult;

  const _QuickPairCard({
    required this.localIp,
    required this.qrPayload,
    required this.hasTransport,
    required this.theme,
    required this.onScanResult,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      color: theme.colorScheme.secondaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Row(
              children: [
                Icon(Icons.qr_code_2,
                    color: theme.colorScheme.onSecondaryContainer),
                const SizedBox(width: 8),
                Text('Quick Pair',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: theme.colorScheme.onSecondaryContainer,
                    )),
                const Spacer(),
                if (localIp != null)
                  Chip(
                    label:
                        Text(localIp!, style: const TextStyle(fontSize: 12)),
                    visualDensity: VisualDensity.compact,
                  ),
              ],
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: SizedBox(
                    height: 48,
                    child: FilledButton.icon(
                      onPressed: () => _showQrDialog(context),
                      icon: const Icon(Icons.qr_code),
                      label: const Text('Show My QR'),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: SizedBox(
                    height: 48,
                    child: FilledButton.tonalIcon(
                      onPressed: () => _openScanner(context),
                      icon: const Icon(Icons.qr_code_scanner),
                      label: const Text('Scan QR'),
                    ),
                  ),
                ),
              ],
            ),
            if (!hasTransport) ...[
              const SizedBox(height: 8),
              Text(
                'Transport layer is not active. QR codes encode '
                'connection info for when transport is enabled.',
                style: TextStyle(
                  color: theme.colorScheme.onSecondaryContainer
                      .withValues(alpha: 0.7),
                  fontSize: 12,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  void _showQrDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('My Connection QR'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(12),
              ),
              child: QrImageView(
                data: qrPayload,
                version: QrVersions.auto,
                size: 220,
              ),
            ),
            const SizedBox(height: 16),
            Text(
              'Scan this QR on the other device\n'
              'to connect both databases instantly.',
              textAlign: TextAlign.center,
              style: TextStyle(
                color: theme.colorScheme.onSurfaceVariant,
                fontSize: 13,
              ),
            ),
            if (localIp != null) ...[
              const SizedBox(height: 8),
              Text('IP: $localIp  Ports: 9400, 9401',
                  style: theme.textTheme.bodySmall),
            ],
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  void _openScanner(BuildContext context) {
    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => _QrScannerPage(onResult: (data) {
          Navigator.of(context).pop();
          onScanResult(data);
        }),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────
// QR Scanner Page
// ─────────────────────────────────────────────────────────────────

class _QrScannerPage extends StatefulWidget {
  final void Function(String data) onResult;
  const _QrScannerPage({required this.onResult});

  @override
  State<_QrScannerPage> createState() => _QrScannerPageState();
}

class _QrScannerPageState extends State<_QrScannerPage> {
  bool _scanned = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Scan Peer QR Code')),
      body: SafeArea(
        child: MobileScanner(
        onDetect: (capture) {
          if (_scanned) return;
          final barcode = capture.barcodes.firstOrNull;
          if (barcode?.rawValue != null) {
            _scanned = true;
            widget.onResult(barcode!.rawValue!);
          }
        },
        ),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────
// How to Connect Card
// ─────────────────────────────────────────────────────────────────

class _HowToConnectCard extends StatelessWidget {
  final ThemeData theme;
  const _HowToConnectCard({required this.theme});

  @override
  Widget build(BuildContext context) {
    return Card(
      color: theme.colorScheme.primaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.link,
                    color: theme.colorScheme.onPrimaryContainer),
                const SizedBox(width: 8),
                Text('How to Link Two Devices',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: theme.colorScheme.onPrimaryContainer,
                    )),
              ],
            ),
            const SizedBox(height: 12),
            _step('1', 'Install this app on both devices'),
            _step('2', 'Connect both to the same WiFi network'),
            _step('3', 'On Device A: tap "Show My QR"'),
            _step('4',
                'On Device B: tap "Scan QR" and point at Device A\'s screen'),
            const SizedBox(height: 8),
            Text(
              'Both databases connect instantly via WebSocket + TLS. '
              'Then use "Mesh" search on the Products tab to query '
              'across devices.\n\n'
              'Devices on the same WiFi also auto-discover each other '
              'via mDNS (~30 seconds).',
              style: TextStyle(
                color: theme.colorScheme.onPrimaryContainer,
                fontSize: 13,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _step(String number, String text) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 12,
            backgroundColor: theme.colorScheme.primary,
            child: Text(number,
                style: TextStyle(
                  color: theme.colorScheme.onPrimary,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                )),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(text,
                style: TextStyle(
                  color: theme.colorScheme.onPrimaryContainer,
                )),
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────
// Database Card
// ─────────────────────────────────────────────────────────────────

class _DatabaseCard extends StatelessWidget {
  final String title;
  final IconData icon;
  final String port;
  final NodeDB db;
  final Map<String, String> stats;
  final ThemeData theme;

  const _DatabaseCard({
    required this.title,
    required this.icon,
    required this.port,
    required this.db,
    required this.stats,
    required this.theme,
  });

  @override
  Widget build(BuildContext context) {
    final transport = db.transport;
    final hasTransport = transport != null;

    List<dynamic> connectedPeers = [];
    List<dynamic> knownPeers = [];
    String shortPeerId = 'N/A';

    if (hasTransport) {
      try {
        final identity = transport.identity();
        final peerId = identity['peer_id']?.toString() ?? 'N/A';
        shortPeerId =
            peerId.length > 12 ? '${peerId.substring(0, 12)}...' : peerId;
        connectedPeers = transport.connectedPeers();
        knownPeers = transport.knownPeers();
      } on Exception {
        // Transport may not be fully initialized
      }
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(icon, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                    child: Text(title, style: theme.textTheme.titleMedium)),
                Chip(
                  avatar: Icon(
                    hasTransport && connectedPeers.isNotEmpty
                        ? Icons.cloud_done
                        : hasTransport
                            ? Icons.cloud_queue
                            : Icons.cloud_off,
                    size: 16,
                    color: hasTransport && connectedPeers.isNotEmpty
                        ? Colors.green
                        : Colors.grey,
                  ),
                  label: Text(hasTransport
                      ? '${connectedPeers.length} peers'
                      : 'local'),
                  visualDensity: VisualDensity.compact,
                ),
              ],
            ),
            const Divider(),
            _row('Mesh', db.mesh?.meshName ?? 'N/A'),
            _row('Database', db.databaseName ?? 'N/A'),
            _row('Sharing', db.sharingStatus),
            _row('Listen Port', port),
            if (hasTransport) _row('Peer ID', shortPeerId),

            if (connectedPeers.isNotEmpty) ...[
              const SizedBox(height: 8),
              Text('Connected Peers:',
                  style: theme.textTheme.labelMedium
                      ?.copyWith(fontWeight: FontWeight.bold)),
              ...connectedPeers.map((p) => Padding(
                    padding: const EdgeInsets.only(left: 8, top: 2),
                    child: Row(
                      children: [
                        const Icon(Icons.circle, size: 8, color: Colors.green),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(p.toString(),
                              style: theme.textTheme.bodySmall,
                              overflow: TextOverflow.ellipsis),
                        ),
                      ],
                    ),
                  )),
            ],

            if (knownPeers.isNotEmpty &&
                knownPeers.length > connectedPeers.length) ...[
              const SizedBox(height: 4),
              Text(
                '${knownPeers.length} peers discovered via mDNS/gossip',
                style:
                    theme.textTheme.bodySmall?.copyWith(color: Colors.grey),
              ),
            ],

            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              children: stats.entries.map((e) {
                return Chip(
                  avatar: Text(e.value,
                      style: TextStyle(
                        color: theme.colorScheme.primary,
                        fontWeight: FontWeight.bold,
                      )),
                  label: Text(e.key),
                  visualDensity: VisualDensity.compact,
                );
              }).toList(),
            ),
          ],
        ),
      ),
    );
  }

  Widget _row(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          SizedBox(
            width: 100,
            child: Text(label,
                style: TextStyle(color: theme.colorScheme.onSurfaceVariant)),
          ),
          Expanded(
            child: Text(value,
                style: const TextStyle(fontWeight: FontWeight.w500)),
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────
// Pending Pairings Card
// ─────────────────────────────────────────────────────────────────

class _PendingPairingsCard extends StatelessWidget {
  final TransportEngine transport;
  final ThemeData theme;
  final VoidCallback onStateChanged;

  const _PendingPairingsCard({
    required this.transport,
    required this.theme,
    required this.onStateChanged,
  });

  @override
  Widget build(BuildContext context) {
    List<Map<String, dynamic>> pending;
    try {
      pending = transport.pendingPairings();
    } on Exception {
      pending = [];
    }

    if (pending.isEmpty) return const SizedBox.shrink();

    return Card(
      color: theme.colorScheme.tertiaryContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.device_unknown,
                    color: theme.colorScheme.onTertiaryContainer),
                const SizedBox(width: 8),
                Text('Pending Pairing Requests',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: theme.colorScheme.onTertiaryContainer,
                    )),
                const Spacer(),
                Badge(
                  label: Text(pending.length.toString()),
                  backgroundColor: theme.colorScheme.error,
                ),
              ],
            ),
            const SizedBox(height: 12),
            ...pending.map((req) => _PendingRequestTile(
                  request: req,
                  transport: transport,
                  theme: theme,
                  onAction: onStateChanged,
                )),
          ],
        ),
      ),
    );
  }
}

class _PendingRequestTile extends StatelessWidget {
  final Map<String, dynamic> request;
  final TransportEngine transport;
  final ThemeData theme;
  final VoidCallback onAction;

  const _PendingRequestTile({
    required this.request,
    required this.transport,
    required this.theme,
    required this.onAction,
  });

  @override
  Widget build(BuildContext context) {
    final peerId = request['peer_id']?.toString() ?? '?';
    final deviceName = request['device_name']?.toString() ?? 'Unknown Device';
    final userId = request['user_id']?.toString() ?? '';
    final shortPeerId =
        peerId.length > 12 ? '${peerId.substring(0, 12)}...' : peerId;
    final shortUserId =
        userId.length > 8 ? '${userId.substring(0, 8)}...' : userId;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(deviceName, style: theme.textTheme.titleSmall),
          const SizedBox(height: 4),
          Text('Peer: $shortPeerId', style: theme.textTheme.bodySmall),
          if (userId.isNotEmpty)
            Text('User: $shortUserId', style: theme.textTheme.bodySmall),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: () {
                  transport.rejectPairing(peerId);
                  onAction();
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Pairing rejected')),
                  );
                },
                child: const Text('Reject'),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: () => _confirmApproval(context, peerId, deviceName),
                child: const Text('Approve'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  void _confirmApproval(
      BuildContext context, String peerId, String deviceName) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Approve Pairing?'),
        content: Text(
          'Allow "$deviceName" to pair with this device?\n\n'
          'Peer ID: ${peerId.length > 24 ? '${peerId.substring(0, 24)}...' : peerId}\n\n'
          'Once approved, this device will reconnect automatically '
          'without needing approval again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.pop(ctx);
              try {
                transport.approvePairing(peerId);
                onAction();
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Paired with $deviceName')),
                );
              } on Exception catch (e) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Pairing failed: $e')),
                );
              }
            },
            child: const Text('Approve'),
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────
// Paired Devices Card
// ─────────────────────────────────────────────────────────────────

class _PairedDevicesCard extends StatelessWidget {
  final TransportEngine transport;
  final ThemeData theme;
  final VoidCallback onStateChanged;

  const _PairedDevicesCard({
    required this.transport,
    required this.theme,
    required this.onStateChanged,
  });

  @override
  Widget build(BuildContext context) {
    List<Map<String, dynamic>> devices;
    try {
      devices = transport.pairedDevices();
    } on Exception {
      devices = [];
    }

    if (devices.isEmpty) return const SizedBox.shrink();

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.devices, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Paired Devices', style: theme.textTheme.titleMedium),
                const Spacer(),
                Text('${devices.length}',
                    style: theme.textTheme.labelLarge
                        ?.copyWith(color: theme.colorScheme.primary)),
              ],
            ),
            const Divider(),
            ...devices.map((device) {
              final peerId = device['peer_id']?.toString() ?? '?';
              final deviceName =
                  device['device_name']?.toString() ?? 'Unknown';
              final userId = device['user_id']?.toString() ?? '';
              final shortPeerId = peerId.length > 12
                  ? '${peerId.substring(0, 12)}...'
                  : peerId;

              return ListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.smartphone, size: 20),
                title: Text(deviceName),
                subtitle: Text(
                  'Peer: $shortPeerId'
                  '${userId.isNotEmpty ? '\nUser: ${userId.length > 8 ? '${userId.substring(0, 8)}...' : userId}' : ''}',
                  style: theme.textTheme.bodySmall,
                ),
                trailing: IconButton(
                  icon: const Icon(Icons.link_off, size: 20),
                  tooltip: 'Unpair',
                  onPressed: () => _confirmUnpair(
                      context, peerId, deviceName),
                ),
              );
            }),
          ],
        ),
      ),
    );
  }

  void _confirmUnpair(
      BuildContext context, String peerId, String deviceName) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Unpair Device?'),
        content: Text(
          'Remove "$deviceName" from paired devices?\n\n'
          'This device will need to be re-paired to connect again.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () {
              Navigator.pop(ctx);
              transport.removePairedDevice(peerId);
              onStateChanged();
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text('Unpaired $deviceName')),
              );
            },
            child: const Text('Unpair'),
          ),
        ],
      ),
    );
  }
}
