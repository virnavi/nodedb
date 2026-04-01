import 'dart:ffi';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

import 'bindings.dart';
import 'types.dart';

/// Extracts error info from [NodeDbErrorStruct] and frees the message.
({int code, String message}) _extractError(
  NodeDbBindings bindings,
  Pointer<NodeDbErrorStruct> errPtr,
) {
  final code = errPtr.ref.code;
  final msgPtr = errPtr.ref.message;
  final message =
      msgPtr == nullptr ? 'unknown error' : msgPtr.cast<Utf8>().toDartString();
  if (msgPtr != nullptr) {
    bindings.freeError(errPtr);
  }
  return (code: code, message: message);
}

/// Execute a query/execute-style FFI call.
///
/// Encodes [request] bytes to native memory, calls [fn], decodes the
/// response, and frees all native buffers. Throws on error.
Uint8List executeRaw(
  NodeDbBindings bindings,
  DartExecuteFn fn,
  int handle,
  Uint8List request,
) {
  final reqPtr = calloc<Uint8>(request.length);
  reqPtr.asTypedList(request.length).setAll(0, request);

  final outResp = calloc<Pointer<Uint8>>();
  final outRespLen = calloc<IntPtr>();
  final outError = calloc<NodeDbErrorStruct>();

  try {
    final success = fn(
      handle,
      reqPtr,
      request.length,
      outResp,
      outRespLen,
      outError,
    );

    if (!success) {
      final err = _extractError(bindings, outError);
      throw NodeDbFfiException(err.code, err.message);
    }

    final responseLen = outRespLen.value;
    if (responseLen == 0) return Uint8List(0);

    final responseBytes = Uint8List.fromList(
      outResp.value.asTypedList(responseLen),
    );
    bindings.freeBuffer(outResp.value, responseLen);

    return responseBytes;
  } finally {
    calloc.free(reqPtr);
    calloc.free(outResp);
    calloc.free(outRespLen);
    calloc.free(outError);
  }
}

/// Execute an open-style FFI call. Returns the engine handle.
int openRaw(
  NodeDbBindings bindings,
  DartOpenFn fn,
  Uint8List config,
) {
  final configPtr = calloc<Uint8>(config.length);
  configPtr.asTypedList(config.length).setAll(0, config);

  final outHandle = calloc<Uint64>();
  final outError = calloc<NodeDbErrorStruct>();

  try {
    final success = fn(configPtr, config.length, outHandle, outError);

    if (!success) {
      final err = _extractError(bindings, outError);
      throw NodeDbFfiException(err.code, err.message);
    }

    return outHandle.value;
  } finally {
    calloc.free(configPtr);
    calloc.free(outHandle);
    calloc.free(outError);
  }
}

/// Execute a write_txn-style FFI call (no response output).
void writeTxnRaw(
  NodeDbBindings bindings,
  DartWriteTxnFn fn,
  int handle,
  Uint8List ops,
) {
  final opsPtr = calloc<Uint8>(ops.length);
  opsPtr.asTypedList(ops.length).setAll(0, ops);

  final outError = calloc<NodeDbErrorStruct>();

  try {
    final success = fn(handle, opsPtr, ops.length, outError);

    if (!success) {
      final err = _extractError(bindings, outError);
      throw NodeDbFfiException(err.code, err.message);
    }
  } finally {
    calloc.free(opsPtr);
    calloc.free(outError);
  }
}

/// Low-level FFI exception thrown by buffer helpers.
///
/// The `nodedb` package maps these to typed [NodeDbException] subclasses.
class NodeDbFfiException implements Exception {
  final int code;
  final String message;

  const NodeDbFfiException(this.code, this.message);

  @override
  String toString() => 'NodeDbFfiException($code): $message';
}
