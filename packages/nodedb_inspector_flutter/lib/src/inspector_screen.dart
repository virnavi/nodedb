import 'package:flutter/material.dart';
import 'package:nodedb_inspector/nodedb_inspector.dart';

import 'inspector_theme.dart';
import 'panel_widget_registry.dart';
import 'panels/dashboard_view.dart';
import 'panels/nosql_view.dart';
import 'panels/graph_view.dart';
import 'panels/vector_view.dart';
import 'panels/federation_view.dart';
import 'panels/dac_view.dart';
import 'panels/provenance_view.dart';
import 'panels/keyresolver_view.dart';
import 'panels/schema_view.dart';
import 'panels/trigger_view.dart';
import 'panels/singleton_view.dart';
import 'panels/preference_view.dart';
import 'panels/access_history_view.dart';
import 'panels/ai_view.dart';

/// Full-screen inspector with tabbed panel navigation.
///
/// Uses [PanelRegistry] to discover available panels dynamically.
/// Custom panels can be registered via [PanelWidgetRegistry].
class InspectorScreen extends StatefulWidget {
  final NodeDbInspector inspector;

  /// Optional widget registry for custom panel rendering.
  final PanelWidgetRegistry? widgetRegistry;

  const InspectorScreen({
    super.key,
    required this.inspector,
    this.widgetRegistry,
  });

  @override
  State<InspectorScreen> createState() => InspectorScreenState();
}

/// State for [InspectorScreen], exposed for testing.
class InspectorScreenState extends State<InspectorScreen> {
  int _selectedIndex = 0;
  late List<_PanelEntry> _panels;

  @override
  void initState() {
    super.initState();
    _panels = _buildPanelList();
  }

  List<_PanelEntry> _buildPanelList() {
    final insp = widget.inspector;
    final panels = <_PanelEntry>[
      _PanelEntry('Dashboard', Icons.dashboard, DashboardView(inspector: insp)),
    ];

    // Built-in widget map: panel ID → widget builder using concrete types
    final builtInWidgets = <String, Widget Function(InspectorPanel)>{
      'nosql': (_) => NoSqlView(entries: insp.databasePanels),
      'schema': (p) => SchemaView(panel: p as SchemaPanel),
      'graph': (p) => GraphView(panel: p as GraphPanel),
      'vector': (p) => VectorView(panel: p as VectorPanel),
      'federation': (p) => FederationView(panel: p as FederationPanel),
      'dac': (p) => DacView(panel: p as DacPanel),
      'provenance': (p) => ProvenanceView(panel: p as ProvenancePanel),
      'keyResolver': (p) => KeyResolverView(panel: p as KeyResolverPanel),
      'triggers': (p) => TriggerView(panel: p as TriggerPanel),
      'singletons': (p) => SingletonView(panel: p as SingletonPanel),
      'preferences': (p) => PreferenceView(panel: p as PreferencePanel),
      'accessHistory': (p) => AccessHistoryView(panel: p as AccessHistoryPanel),
      'ai': (p) => AiView(panel: p as AiPanel),
    };

    for (final panel in insp.panelRegistry.available) {
      final id = panel.descriptor.id;
      final icon = PanelWidgetRegistry.iconFromHint(panel.descriptor.iconHint);
      final label = panel.descriptor.displayName;

      // Try built-in widget, then custom registry, skip if no widget
      final builtIn = builtInWidgets[id];
      if (builtIn != null) {
        panels.add(_PanelEntry(label, icon, builtIn(panel)));
        continue;
      }

      final custom = widget.widgetRegistry?.build(panel);
      if (custom != null) {
        panels.add(_PanelEntry(label, icon, custom));
      }
    }

    return panels;
  }

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: inspectorTheme(),
      child: Scaffold(
        appBar: AppBar(
          title: const Text('NodeDB Inspector'),
          leading: IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
          ),
          actions: [
            IconButton(
              icon: const Icon(Icons.refresh),
              tooltip: 'Refresh',
              onPressed: () => setState(() {
                _panels = _buildPanelList();
              }),
            ),
          ],
        ),
        body: Row(
          children: [
            NavigationRail(
              selectedIndex: _selectedIndex,
              onDestinationSelected: (i) => setState(() => _selectedIndex = i),
              labelType: NavigationRailLabelType.all,
              destinations: [
                for (final panel in _panels)
                  NavigationRailDestination(
                    icon: Icon(panel.icon),
                    label: Text(panel.label),
                  ),
              ],
            ),
            const VerticalDivider(width: 1),
            Expanded(
              child: _panels[_selectedIndex].widget,
            ),
          ],
        ),
      ),
    );
  }
}

class _PanelEntry {
  final String label;
  final IconData icon;
  final Widget widget;
  const _PanelEntry(this.label, this.icon, this.widget);
}
