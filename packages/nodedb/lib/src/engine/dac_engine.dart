import 'package:nodedb_ffi/nodedb_ffi.dart';

import '../error/nodedb_error.dart';
import '../model/access_rule.dart';
import '../util/msgpack.dart';

/// Typed wrapper for the NodeDB DAC (Data Access Control) engine.
class DacEngine {
  final int _handle;
  final NodeDbBindings _bindings;

  DacEngine._(this._handle, this._bindings);

  /// Attach to an existing DAC engine handle (for multi-isolate use).
  factory DacEngine.fromHandle(NodeDbBindings bindings, int handle) {
    return DacEngine._(handle, bindings);
  }

  static DacEngine open(NodeDbBindings bindings, String path) {
    final handle = openRaw(bindings, bindings.dacOpen, buildConfig(path));
    return DacEngine._(handle, bindings);
  }

  int get handle => _handle;

  AccessRule addRule({
    required String collection,
    String? field,
    String? recordId,
    required String subjectType,
    required String subjectId,
    required String permission,
    String? expiresAt,
  }) {
    final resp = _execute({
      'action': 'add_rule',
      'collection': collection,
      if (field != null) 'field': field,
      if (recordId != null) 'record_id': recordId,
      'subject_type': subjectType,
      'subject_id': subjectId,
      'permission': permission,
      if (expiresAt != null) 'expires_at': expiresAt,
    });
    return AccessRule.fromMsgpack(resp);
  }

  AccessRule? getRule(int id) {
    final resp = _execute({'action': 'get_rule', 'id': id});
    if (resp == null) return null;
    return AccessRule.fromMsgpack(resp);
  }

  void updateRule(int id, {String? permission, String? expiresAt}) {
    _execute({
      'action': 'update_rule',
      'id': id,
      if (permission != null) 'permission': permission,
      if (expiresAt != null) 'expires_at': expiresAt,
    });
  }

  void deleteRule(int id) {
    _execute({'action': 'delete_rule', 'id': id});
  }

  List<AccessRule> allRules() {
    final resp = _execute({'action': 'all_rules'});
    if (resp is! List) return [];
    return resp.map((r) => AccessRule.fromMsgpack(r)).toList();
  }

  List<AccessRule> rulesForCollection(String collection) {
    final resp = _execute({
      'action': 'rules_for_collection',
      'collection': collection,
    });
    if (resp is! List) return [];
    return resp.map((r) => AccessRule.fromMsgpack(r)).toList();
  }

  int ruleCount() {
    final resp = _execute({'action': 'rule_count'});
    return (resp is int) ? resp : 0;
  }

  Map<String, dynamic>? filterDocument({
    required String collection,
    required Map<String, dynamic> document,
    required String peerId,
    List<String>? groupIds,
    String? recordId,
  }) {
    final resp = _execute({
      'action': 'filter_document',
      'collection': collection,
      'document': document,
      'peer_id': peerId,
      if (groupIds != null) 'group_ids': groupIds,
      if (recordId != null) 'record_id': recordId,
    });
    if (resp is Map) return Map<String, dynamic>.from(resp);
    return null;
  }

  void close() {
    _bindings.dacClose(_handle);
  }

  dynamic _execute(Map<String, dynamic> request) {
    try {
      final bytes = executeRaw(
        _bindings,
        _bindings.dacExecute,
        _handle,
        msgpackEncode(request),
      );
      return msgpackDecode(bytes);
    } on NodeDbFfiException catch (e) {
      throw NodeDbException.fromCode(e.code, e.message);
    }
  }
}
