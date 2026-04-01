import 'package:inspector_sdk/inspector_sdk.dart';
import 'package:nodedb/nodedb.dart';

import 'inspector_config.dart';
import 'panels/nosql_panel.dart';
import 'panels/graph_panel.dart';
import 'panels/vector_panel.dart';
import 'panels/federation_panel.dart';
import 'panels/dac_panel.dart';
import 'panels/provenance_panel.dart';
import 'panels/keyresolver_panel.dart';
import 'panels/schema_panel.dart';
import 'panels/trigger_panel.dart';
import 'panels/singleton_panel.dart';
import 'panels/preference_panel.dart';
import 'panels/access_history_panel.dart';
import 'panels/ai_panel.dart';
import 'server/inspector_server.dart';

/// Debug inspector for NodeDB databases.
///
/// Provides structured access to all engine data for debugging UIs.
/// Each engine gets its own panel provider, registered in a [PanelRegistry].
///
/// ```dart
/// final inspector = NodeDbInspector(db);
/// final snap = inspector.snapshot();
/// print(snap['nosql']['totalDocuments']);
/// ```
/// Descriptor for a database within a mesh, used by inspector panels.
class DatabaseEntry {
  final String mesh;
  final String database;
  final NoSqlPanel panel;

  const DatabaseEntry({
    required this.mesh,
    required this.database,
    required this.panel,
  });
}

class NodeDbInspector {
  final NodeDB db;
  final InspectorConfig config;

  /// All databases registered with the inspector (for multi-DB panels).
  final List<NodeDB> databases;

  /// Panel registry for plugin-based panel discovery.
  final PanelRegistry panelRegistry = PanelRegistry();

  /// Data source registry for plugin-based data source discovery.
  final DataSourceRegistry dataSourceRegistry = DataSourceRegistry();

  InspectorServer? _server;

  NodeDbInspector(
    this.db, {
    this.config = const InspectorConfig(),
    VectorOpenConfig? vectorConfig,
  }) : databases = [db] {
    _registerBuiltInPanels(vectorConfig);
  }

  /// Create an inspector spanning multiple databases in a mesh.
  NodeDbInspector.mesh(
    this.databases, {
    this.config = const InspectorConfig(),
    VectorOpenConfig? vectorConfig,
  }) : db = databases.first {
    _registerBuiltInPanels(vectorConfig);
  }

  void _registerBuiltInPanels(VectorOpenConfig? vectorConfig) {
    panelRegistry.register(NoSqlPanel(db.nosql));
    panelRegistry.register(SchemaPanel(db.nosql, db.provenance));
    panelRegistry.register(GraphPanel(db.graph));
    panelRegistry.register(VectorPanel(db.vector, vectorConfig));
    panelRegistry.register(FederationPanel(db.federation));
    panelRegistry.register(DacPanel(db.dac));
    panelRegistry.register(ProvenancePanel(db.provenance));
    panelRegistry.register(KeyResolverPanel(db.keyResolver));
    panelRegistry.register(TriggerPanel(db.nosql));
    panelRegistry.register(SingletonPanel(db.nosql));
    panelRegistry.register(PreferencePanel(db.nosql));
    panelRegistry.register(AccessHistoryPanel(db.nosql));
    panelRegistry.register(AiPanel(db.provenance, db.aiProvenance, db.aiQuery));
  }

  /// Register a custom plugin.
  void registerPlugin(InspectorPlugin plugin) {
    plugin.register(panelRegistry, dataSourceRegistry);
  }

  // ── Typed accessors (backward compatibility) ──────────────────

  NoSqlPanel get nosql => panelRegistry.getAs<NoSqlPanel>('nosql')!;

  GraphPanel? get graph {
    final p = panelRegistry.getAs<GraphPanel>('graph');
    return p?.isAvailable == true ? p : null;
  }

  VectorPanel? get vector {
    final p = panelRegistry.getAs<VectorPanel>('vector');
    return p?.isAvailable == true ? p : null;
  }

  FederationPanel get federation =>
      panelRegistry.getAs<FederationPanel>('federation')!;

  DacPanel? get dac {
    final p = panelRegistry.getAs<DacPanel>('dac');
    return p?.isAvailable == true ? p : null;
  }

  ProvenancePanel? get provenance {
    final p = panelRegistry.getAs<ProvenancePanel>('provenance');
    return p?.isAvailable == true ? p : null;
  }

  KeyResolverPanel? get keyResolver {
    final p = panelRegistry.getAs<KeyResolverPanel>('keyResolver');
    return p?.isAvailable == true ? p : null;
  }

  SchemaPanel get schema => panelRegistry.getAs<SchemaPanel>('schema')!;

  TriggerPanel get triggers =>
      panelRegistry.getAs<TriggerPanel>('triggers')!;

  SingletonPanel get singletons =>
      panelRegistry.getAs<SingletonPanel>('singletons')!;

  PreferencePanel get preferences =>
      panelRegistry.getAs<PreferencePanel>('preferences')!;

  AccessHistoryPanel get accessHistory =>
      panelRegistry.getAs<AccessHistoryPanel>('accessHistory')!;

  AiPanel? get ai {
    final p = panelRegistry.getAs<AiPanel>('ai');
    return p?.isAvailable == true ? p : null;
  }

  // ── Data access ───────────────────────────────────────────────

  /// Returns all databases with their mesh/database names and NoSQL panels.
  List<DatabaseEntry> get databasePanels {
    return databases.map((d) {
      final mesh = d.mesh?.meshName ?? 'default';
      final database = d.databaseName ?? 'unnamed';
      return DatabaseEntry(
        mesh: mesh,
        database: database,
        panel: NoSqlPanel(d.nosql),
      );
    }).toList();
  }

  /// Returns a full database summary across all enabled engines.
  Map<String, dynamic> snapshot() {
    final snap = <String, dynamic>{
      'version': db.ffiVersion,
    };
    for (final panel in panelRegistry.available) {
      snap[panel.descriptor.id] = panel.summary();
    }
    return snap;
  }

  /// Start the debug inspector server.
  Future<void> start() async {
    if (_server != null) return;
    _server = InspectorServer(
      this,
      port: config.port,
      passcode: config.passcode,
      snapshotInterval: config.cacheTtl,
    );
    await _server!.start();
  }

  /// Stop the debug inspector server.
  Future<void> stop() async {
    await _server?.stop();
    _server = null;
  }

  /// Whether the debug inspector server is running.
  bool get isRunning => _server?.isRunning ?? false;

  /// Returns a list of enabled panel names.
  List<String> enabledPanels() => panelRegistry.enabledPanelIds();
}
