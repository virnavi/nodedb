/// NodeDB embedded multi-engine database for Flutter.
library nodedb;

// Annotations
export 'src/annotations/annotations.dart';

// Error types
export 'src/error/error_codes.dart';
export 'src/error/nodedb_error.dart';

// Models
export 'src/model/document.dart';
export 'src/model/graph_node.dart';
export 'src/model/graph_edge.dart';
export 'src/model/vector_record.dart';
export 'src/model/search_result.dart';
export 'src/model/node_peer.dart';
export 'src/model/node_group.dart';
export 'src/model/access_rule.dart';
export 'src/model/provenance_envelope.dart';
export 'src/model/key_entry.dart';
export 'src/model/migration.dart';
export 'src/model/keypair.dart';
export 'src/model/signer.dart';
export 'src/model/mesh_config.dart';
export 'src/model/transport_config.dart';
export 'src/model/access_history.dart';
export 'src/model/trim.dart';
export 'src/model/ai_provenance.dart';
export 'src/model/ai_query.dart';
export 'src/model/cache_config.dart';

// Engines
export 'src/engine/nosql_engine.dart';
export 'src/engine/graph_engine.dart';
export 'src/engine/vector_engine.dart';
export 'src/engine/federation_engine.dart';
export 'src/engine/dac_engine.dart';
export 'src/engine/transport_engine.dart';
export 'src/engine/provenance_engine.dart';
export 'src/engine/keyresolver_engine.dart';
export 'src/engine/ai_provenance_engine.dart';
export 'src/engine/ai_query_engine.dart';

// Schema
export 'src/schema/schema_types.dart';

// Query
export 'src/query/filter_query.dart';
export 'src/query/query_result.dart';
export 'src/query/collection_query.dart';

// Adapters
export 'src/adapter/ai_provenance_adapter.dart';
export 'src/adapter/ai_query_adapter.dart';

// Utilities
export 'src/util/id_generator.dart';

// Sync
export 'src/sync/collection_notifier.dart';

// P2P
export 'src/p2p/p2p_message_store.dart';

// Mesh
export 'src/database_mesh.dart';

// Top-level
export 'src/nodedb_base.dart';
