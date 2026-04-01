import 'package:uuid/uuid.dart';

const _uuid = Uuid();

/// Generate a UUID v7 (time-ordered) identifier.
///
/// Used by generated DAOs to auto-assign IDs for String-id models.
String generateNodeDbId() => _uuid.v7();
