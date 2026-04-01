/// Type of access event recorded in the access history.
enum AccessEventType {
  read,
  write,
  watch,
  federatedRead,
  aiWrite;

  String get value {
    switch (this) {
      case AccessEventType.read:
        return 'read';
      case AccessEventType.write:
        return 'write';
      case AccessEventType.watch:
        return 'watch';
      case AccessEventType.federatedRead:
        return 'federated_read';
      case AccessEventType.aiWrite:
        return 'ai_write';
    }
  }

  static AccessEventType? fromString(String s) {
    switch (s) {
      case 'read':
        return AccessEventType.read;
      case 'write':
        return AccessEventType.write;
      case 'watch':
        return AccessEventType.watch;
      case 'federated_read':
        return AccessEventType.federatedRead;
      case 'ai_write':
        return AccessEventType.aiWrite;
      default:
        return null;
    }
  }
}

/// Scope under which the access occurred.
enum QueryScope {
  local,
  federated,
  aiQuery,
  federatedAndAi;

  String get value {
    switch (this) {
      case QueryScope.local:
        return 'local';
      case QueryScope.federated:
        return 'federated';
      case QueryScope.aiQuery:
        return 'ai_query';
      case QueryScope.federatedAndAi:
        return 'federated_and_ai';
    }
  }

  static QueryScope? fromString(String s) {
    switch (s) {
      case 'local':
        return QueryScope.local;
      case 'federated':
        return QueryScope.federated;
      case 'ai_query':
        return QueryScope.aiQuery;
      case 'federated_and_ai':
        return QueryScope.federatedAndAi;
      default:
        return null;
    }
  }
}

/// Configuration for access history behaviour.
class AccessHistoryConfig {
  /// How long to retain access history entries (in seconds). Default: 365 days.
  final int retentionPeriodSecs;

  /// Whether to automatically trim old entries. Default: true.
  final bool autoTrim;

  /// Interval between auto-trim runs (in seconds). Default: 24 hours.
  final int autoTrimIntervalSecs;

  /// Collections to exclude from access history recording.
  final List<String> excludeCollections;

  /// Whether to track watch events. Default: false.
  final bool trackWatchEvents;

  const AccessHistoryConfig({
    this.retentionPeriodSecs = 365 * 24 * 3600,
    this.autoTrim = true,
    this.autoTrimIntervalSecs = 24 * 3600,
    this.excludeCollections = const ['__access_history__'],
    this.trackWatchEvents = false,
  });
}
