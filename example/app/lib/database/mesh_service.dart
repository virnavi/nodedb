import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_example/database/user_database.dart';
import 'package:nodedb_example/database/product_database.dart';

/// Coordinates multiple domain databases in a single mesh.
///
/// Creates a shared [DatabaseMesh] that owns the transport configuration
/// and federation engine. Both databases communicate with remote devices
/// exclusively through the mesh.
class MeshService {
  late final DatabaseMesh mesh;
  late final UserDatabase userDb;
  late final ProductDatabase productDb;

  /// All NodeDB instances for inspector overlay.
  List<NodeDB> get databases => [userDb.db, productDb.db];

  void init(String baseDir) {
    final meshDir = Directory('$baseDir/mesh');
    if (!meshDir.existsSync()) meshDir.createSync(recursive: true);

    mesh = DatabaseMesh.open(
      directory: meshDir.path,
      config: const MeshConfig(meshName: 'nodedb-example'),
      transportConfig: const TransportConfig(
        listenAddr: '0.0.0.0:9400',
        mdnsEnabled: true,
      ),
    );

    userDb = UserDatabase()..init(baseDir, mesh);
    productDb = ProductDatabase()..init(baseDir, mesh);

    // Seed data
    userDb.seedIfEmpty();
    productDb.seedIfEmpty(userDb.users.findAll());
  }

  /// Search products across the mesh (local + federated peers).
  ///
  /// Returns results tagged with source peer ID.
  /// On a single device, only local results are returned.
  /// On multiple devices in the same mesh, remote results appear too.
  List<FederatedResult<Document>> searchProductsFederated(String query) {
    return productDb.db.findAllFederated(
      'public.products',
      filter: {
        'Condition': {
          'Contains': {'field': 'name', 'value': query}
        }
      },
    );
  }
}
