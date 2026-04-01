import 'dart:io';

import 'package:nodedb/nodedb.dart';
import 'package:nodedb_example/models/user_models.dart';

/// User domain database — manages users and preferences.
///
/// Participates in the mesh with `databaseName: 'users'`.
class UserDatabase {
  late final NodeDB db;
  late final UserDao users;
  late final UserPrefsPrefs prefs;

  void init(String baseDir, DatabaseMesh mesh) {
    final dir = Directory('$baseDir/users');
    if (!dir.existsSync()) dir.createSync(recursive: true);

    db = NodeDB.open(
      directory: dir.path,
      databaseName: 'users',
      mesh: mesh,
      provenanceEnabled: true,
    );

    users = UserDao(db.nosql, db.provenance, db.notifier);
    prefs = UserPrefsPrefs(db.nosql);
  }

  /// Seed sample users if empty.
  void seedIfEmpty() {
    if (users.count() > 0) return;

    users.createAll([
      User(name: 'Alice', email: 'alice@example.com'),
      User(name: 'Bob', email: 'bob@example.com'),
      User(name: 'Charlie', email: 'charlie@example.com'),
    ]);

    prefs.setTheme('system');
    prefs.setNotificationsEnabled(true);
  }
}
