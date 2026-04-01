import 'dart:typed_data';

import 'package:msgpack_dart/msgpack_dart.dart' as msgpack;

/// Encode a Dart value to MessagePack bytes.
Uint8List msgpackEncode(dynamic value) {
  return msgpack.serialize(value);
}

/// Decode MessagePack bytes to a Dart value.
dynamic msgpackDecode(Uint8List bytes) {
  if (bytes.isEmpty) return null;
  return msgpack.deserialize(bytes);
}

/// Build a MessagePack-encoded request map with an 'action' field.
Uint8List buildRequest(String action, [Map<String, dynamic>? fields]) {
  final map = <String, dynamic>{'action': action};
  if (fields != null) map.addAll(fields);
  return msgpackEncode(map);
}

/// Build a MessagePack-encoded config map with a 'path' field.
Uint8List buildConfig(String path, [Map<String, dynamic>? extra]) {
  final map = <String, dynamic>{'path': path};
  if (extra != null) map.addAll(extra);
  return msgpackEncode(map);
}

/// Decode a field from either a Map or a positional List.
///
/// Rust's `rmpv::ext::to_value` produces positional arrays for structs,
/// while some FFI responses are manually-built maps. This helper handles both.
dynamic decodeField(dynamic decoded, String key, int positionalIndex) {
  if (decoded is Map) return decoded[key];
  if (decoded is List && positionalIndex < decoded.length) {
    return decoded[positionalIndex];
  }
  return null;
}
