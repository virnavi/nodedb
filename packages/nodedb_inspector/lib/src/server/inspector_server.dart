import 'dart:async';
import 'dart:convert';
import 'dart:io';

import '../inspector.dart';
import 'command_router.dart';
import 'inspector_html.dart';

/// HTTP + WebSocket server for the NodeDB Debug Inspector.
///
/// Serves the web UI at `GET /` and accepts WebSocket connections at `/ws`.
/// Binds to `localhost` only for security.
class InspectorServer {
  final NodeDbInspector _inspector;
  final int _requestedPort;
  final String? passcode;
  final Duration snapshotInterval;

  HttpServer? _server;
  final List<_AuthenticatedClient> _clients = [];
  Timer? _snapshotTimer;
  late final CommandRouter _router;

  InspectorServer(
    this._inspector, {
    int port = 8110,
    this.passcode,
    this.snapshotInterval = const Duration(seconds: 5),
  }) : _requestedPort = port {
    _router = CommandRouter(_inspector);
  }

  /// The port the server is bound to. Returns the requested port before start.
  int get port => _server?.port ?? _requestedPort;

  /// Start the server on localhost.
  Future<void> start() async {
    _server = await HttpServer.bind(InternetAddress.loopbackIPv4, _requestedPort);
    _server!.listen(_handleRequest);
    _startSnapshotPush();
  }

  /// Stop the server and disconnect all clients.
  Future<void> stop() async {
    _snapshotTimer?.cancel();
    _snapshotTimer = null;
    for (final client in List.of(_clients)) {
      try {
        await client.socket.close();
      } catch (_) {}
    }
    _clients.clear();
    await _server?.close();
    _server = null;
  }

  /// Whether the server is currently running.
  bool get isRunning => _server != null;

  void _handleRequest(HttpRequest request) {
    if (WebSocketTransformer.isUpgradeRequest(request)) {
      _upgradeWebSocket(request);
    } else if (request.uri.path == '/' && request.method == 'GET') {
      _serveHtml(request);
    } else {
      request.response
        ..statusCode = HttpStatus.notFound
        ..write('Not found')
        ..close();
    }
  }

  void _serveHtml(HttpRequest request) {
    request.response
      ..statusCode = HttpStatus.ok
      ..headers.contentType = ContentType.html
      ..write(inspectorHtml)
      ..close();
  }

  Future<void> _upgradeWebSocket(HttpRequest request) async {
    final socket = await WebSocketTransformer.upgrade(request);
    final client = _AuthenticatedClient(socket, authenticated: passcode == null);

    socket.listen(
      (data) => _handleMessage(client, data),
      onDone: () => _clients.remove(client),
      onError: (_) => _clients.remove(client),
    );

    if (client.authenticated) {
      _clients.add(client);
      _sendSnapshot(client);
    }
  }

  void _handleMessage(_AuthenticatedClient client, dynamic data) {
    if (data is! String) return;

    Map<String, dynamic> msg;
    try {
      msg = jsonDecode(data) as Map<String, dynamic>;
    } catch (_) {
      _sendError(client, 'invalid_json', 'Could not parse message');
      return;
    }

    // Auth flow
    if (!client.authenticated) {
      if (msg['auth'] == passcode) {
        client.authenticated = true;
        _clients.add(client);
        _send(client, {'ok': true});
        _sendSnapshot(client);
      } else {
        _send(client, {'ok': false, 'error': 'bad_passcode'});
        try {
          client.socket.close();
        } catch (_) {}
      }
      return;
    }

    // Command dispatch
    final cmd = msg['cmd'] as String?;
    if (cmd == null) {
      _sendError(client, 'missing_cmd', 'Message must have a "cmd" field');
      return;
    }

    try {
      final result = _router.dispatch(cmd, msg);
      _send(client, {'cmd': cmd, 'data': result});
    } catch (e) {
      _sendError(client, 'command_error', e.toString());
    }
  }

  void _send(_AuthenticatedClient client, Map<String, dynamic> payload) {
    try {
      client.socket.add(jsonEncode(payload));
    } catch (_) {
      _clients.remove(client);
    }
  }

  void _sendError(_AuthenticatedClient client, String code, String message) {
    _send(client, {'error': code, 'message': message});
  }

  void _sendSnapshot(_AuthenticatedClient client) {
    try {
      final snap = _inspector.snapshot();
      _send(client, {'cmd': 'snapshot', 'data': snap});
    } catch (_) {}
  }

  void _startSnapshotPush() {
    _snapshotTimer = Timer.periodic(snapshotInterval, (_) {
      if (_clients.isEmpty) return;
      try {
        final snap = _inspector.snapshot();
        final payload = jsonEncode({'cmd': 'snapshot', 'data': snap});
        for (final client in List.of(_clients)) {
          try {
            client.socket.add(payload);
          } catch (_) {
            _clients.remove(client);
          }
        }
      } catch (_) {}
    });
  }
}

class _AuthenticatedClient {
  final WebSocket socket;
  bool authenticated;
  _AuthenticatedClient(this.socket, {this.authenticated = false});
}
