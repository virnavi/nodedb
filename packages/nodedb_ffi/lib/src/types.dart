import 'dart:ffi';

import 'package:ffi/ffi.dart';

/// FFI struct matching Rust's `NodeDbError` in nodedb-ffi/src/types.rs.
final class NodeDbErrorStruct extends Struct {
  @Int32()
  external int code;

  external Pointer<Utf8> message;
}
