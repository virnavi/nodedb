/// User domain models — Users DB.
///
/// Models: User, UserPrefs
/// Database: meshName='nodedb-example', databaseName='users'
library;

import 'package:nodedb/nodedb.dart';

part 'user_models.g.dart';

// ─────────────────────────────────────────────────────────────────
// @collection — User
// ─────────────────────────────────────────────────────────────────

@collection
class User {
  String id = '';
  late String name;
  @Index(unique: true)
  late String email;
  String? avatarUrl;
  DateTime? createdAt;

  User({
    this.id = '',
    required this.name,
    required this.email,
    this.avatarUrl,
    this.createdAt,
  });
}

// ─────────────────────────────────────────────────────────────────
// @preferences — UserPrefs (encrypted per-key storage)
// ─────────────────────────────────────────────────────────────────

@preferences
class UserPrefs {
  String theme;
  bool notificationsEnabled;

  UserPrefs({
    this.theme = 'system',
    this.notificationsEnabled = true,
  });
}
