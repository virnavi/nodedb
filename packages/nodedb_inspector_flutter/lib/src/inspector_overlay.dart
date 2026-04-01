import 'package:flutter/material.dart';
import 'package:nodedb/nodedb.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import 'inspector_screen.dart';

/// A floating debug overlay that wraps your app widget.
///
/// When [enabled] is true, displays a small floating action button
/// in the bottom-right corner. Tapping it opens the full inspector screen.
///
/// ```dart
/// NodeInspectorOverlay(
///   db: myNodeDB,
///   enabled: kDebugMode,
///   child: MyApp(),
/// )
/// ```
class NodeInspectorOverlay extends StatefulWidget {
  /// The NodeDB instance to inspect.
  final NodeDB db;

  /// The app widget to wrap.
  final Widget child;

  /// Whether the overlay is active. Set to `kDebugMode` for debug-only.
  final bool enabled;

  /// Optional inspector configuration (port, passcode, cache TTL).
  final InspectorConfig config;

  /// Position of the floating button from the bottom-right corner.
  final Offset buttonOffset;

  const NodeInspectorOverlay({
    super.key,
    required this.db,
    required this.child,
    this.enabled = true,
    this.config = const InspectorConfig(),
    this.buttonOffset = const Offset(16, 16),
  });

  @override
  State<NodeInspectorOverlay> createState() => NodeInspectorOverlayState();
}

/// State for [NodeInspectorOverlay], exposed for testing.
class NodeInspectorOverlayState extends State<NodeInspectorOverlay> {
  late NodeDbInspector _inspector;

  @override
  void initState() {
    super.initState();
    if (widget.enabled) {
      _inspector = NodeDbInspector(widget.db, config: widget.config);
    }
  }

  @override
  void didUpdateWidget(NodeInspectorOverlay oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.db != oldWidget.db || widget.config != oldWidget.config) {
      _inspector = NodeDbInspector(widget.db, config: widget.config);
    }
  }

  /// Opens the inspector screen programmatically.
  void openInspector() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => InspectorScreen(inspector: _inspector),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.enabled) return widget.child;

    return Directionality(
      textDirection: TextDirection.ltr,
      child: Stack(
        children: [
          widget.child,
          Positioned(
            right: widget.buttonOffset.dx,
            bottom: widget.buttonOffset.dy,
            child: _InspectorButton(onPressed: openInspector),
          ),
        ],
      ),
    );
  }
}

class _InspectorButton extends StatelessWidget {
  final VoidCallback onPressed;
  const _InspectorButton({required this.onPressed});

  @override
  Widget build(BuildContext context) {
    return Material(
      type: MaterialType.transparency,
      child: SizedBox(
        width: 48,
        height: 48,
        child: FloatingActionButton(
          heroTag: '__nodedb_inspector__',
          mini: true,
          backgroundColor: const Color(0xFF7aa2f7),
          foregroundColor: const Color(0xFF1a1b26),
          onPressed: onPressed,
          tooltip: 'NodeDB Inspector',
          child: const Icon(Icons.bug_report, size: 20),
        ),
      ),
    );
  }
}
