import 'dart:ffi';

import 'types.dart';

// ── Native type definitions ─────────────────────────────────────────

// Open: (config_ptr, config_len, out_handle, out_error) -> bool
typedef _NativeOpenFn = Bool Function(
  Pointer<Uint8> configPtr,
  IntPtr configLen,
  Pointer<Uint64> outHandle,
  Pointer<NodeDbErrorStruct> outError,
);
typedef DartOpenFn = bool Function(
  Pointer<Uint8> configPtr,
  int configLen,
  Pointer<Uint64> outHandle,
  Pointer<NodeDbErrorStruct> outError,
);

// Close: (handle) -> void
typedef _NativeCloseFn = Void Function(Uint64 handle);
typedef DartCloseFn = void Function(int handle);

// Execute: (handle, req_ptr, req_len, out_resp, out_resp_len, out_error) -> bool
typedef _NativeExecuteFn = Bool Function(
  Uint64 handle,
  Pointer<Uint8> reqPtr,
  IntPtr reqLen,
  Pointer<Pointer<Uint8>> outResp,
  Pointer<IntPtr> outRespLen,
  Pointer<NodeDbErrorStruct> outError,
);
typedef DartExecuteFn = bool Function(
  int handle,
  Pointer<Uint8> reqPtr,
  int reqLen,
  Pointer<Pointer<Uint8>> outResp,
  Pointer<IntPtr> outRespLen,
  Pointer<NodeDbErrorStruct> outError,
);

// WriteTxn: (handle, ops_ptr, ops_len, out_error) -> bool
typedef _NativeWriteTxnFn = Bool Function(
  Uint64 handle,
  Pointer<Uint8> opsPtr,
  IntPtr opsLen,
  Pointer<NodeDbErrorStruct> outError,
);
typedef DartWriteTxnFn = bool Function(
  int handle,
  Pointer<Uint8> opsPtr,
  int opsLen,
  Pointer<NodeDbErrorStruct> outError,
);

// LinkTransport: (db_handle, transport_handle, out_error) -> bool
typedef _NativeLinkTransportFn = Bool Function(
  Uint64 dbHandle,
  Uint64 transportHandle,
  Pointer<NodeDbErrorStruct> outError,
);
typedef DartLinkTransportFn = bool Function(
  int dbHandle,
  int transportHandle,
  Pointer<NodeDbErrorStruct> outError,
);

// FfiVersion: () -> uint32
typedef _NativeVersionFn = Uint32 Function();
typedef DartVersionFn = int Function();

// FreeBuffer: (ptr, len) -> void
typedef _NativeFreeBufferFn = Void Function(Pointer<Uint8> ptr, IntPtr len);
typedef DartFreeBufferFn = void Function(Pointer<Uint8> ptr, int len);

// FreeError: (error) -> void
typedef _NativeFreeErrorFn = Void Function(Pointer<NodeDbErrorStruct> error);
typedef DartFreeErrorFn = void Function(Pointer<NodeDbErrorStruct> error);

// ── Bindings class ──────────────────────────────────────────────────

/// Raw dart:ffi bindings for all 35+ NodeDB FFI functions.
class NodeDbBindings {
  final DynamicLibrary _lib;

  NodeDbBindings(this._lib);

  // ── Utility ─────────────────────────────────────────────────────

  late final DartVersionFn ffiVersion = _lib
      .lookupFunction<_NativeVersionFn, DartVersionFn>('nodedb_ffi_version');

  late final DartFreeBufferFn freeBuffer = _lib
      .lookupFunction<_NativeFreeBufferFn, DartFreeBufferFn>(
          'nodedb_free_buffer');

  late final DartFreeErrorFn freeError = _lib
      .lookupFunction<_NativeFreeErrorFn, DartFreeErrorFn>(
          'nodedb_free_error');

  // ── NoSQL ───────────────────────────────────────────────────────

  late final DartOpenFn open =
      _lib.lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_open');

  late final DartCloseFn close =
      _lib.lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_close');

  late final DartExecuteFn query =
      _lib.lookupFunction<_NativeExecuteFn, DartExecuteFn>('nodedb_query');

  late final DartWriteTxnFn writeTxn = _lib
      .lookupFunction<_NativeWriteTxnFn, DartWriteTxnFn>('nodedb_write_txn');

  late final DartExecuteFn dbExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>('nodedb_db_execute');

  // ── Graph ───────────────────────────────────────────────────────

  late final DartOpenFn graphOpen =
      _lib.lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_graph_open');

  late final DartCloseFn graphClose =
      _lib.lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_graph_close');

  late final DartExecuteFn graphExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_graph_execute');

  // ── Vector ──────────────────────────────────────────────────────

  late final DartOpenFn vectorOpen =
      _lib.lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_vector_open');

  late final DartCloseFn vectorClose =
      _lib.lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_vector_close');

  late final DartExecuteFn vectorExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_vector_execute');

  // ── Federation ──────────────────────────────────────────────────

  late final DartOpenFn federationOpen = _lib
      .lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_federation_open');

  late final DartCloseFn federationClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_federation_close');

  late final DartExecuteFn federationExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_federation_execute');

  // ── DAC ─────────────────────────────────────────────────────────

  late final DartOpenFn dacOpen =
      _lib.lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_dac_open');

  late final DartCloseFn dacClose =
      _lib.lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_dac_close');

  late final DartExecuteFn dacExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_dac_execute');

  // ── Transport ───────────────────────────────────────────────────

  late final DartOpenFn transportOpen = _lib
      .lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_transport_open');

  late final DartCloseFn transportClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_transport_close');

  late final DartExecuteFn transportExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_transport_execute');

  late final DartLinkTransportFn linkTransport = _lib
      .lookupFunction<_NativeLinkTransportFn, DartLinkTransportFn>(
          'nodedb_link_transport');

  // ── Provenance ──────────────────────────────────────────────────

  late final DartOpenFn provenanceOpen = _lib
      .lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_provenance_open');

  late final DartCloseFn provenanceClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_provenance_close');

  late final DartExecuteFn provenanceExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_provenance_execute');

  // ── Key Resolver ────────────────────────────────────────────────

  late final DartOpenFn keyresolverOpen = _lib
      .lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_keyresolver_open');

  late final DartCloseFn keyresolverClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>(
          'nodedb_keyresolver_close');

  late final DartExecuteFn keyresolverExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_keyresolver_execute');

  // ── AI Provenance ───────────────────────────────────────────────

  late final DartOpenFn aiProvenanceOpen = _lib
      .lookupFunction<_NativeOpenFn, DartOpenFn>(
          'nodedb_ai_provenance_open');

  late final DartCloseFn aiProvenanceClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>(
          'nodedb_ai_provenance_close');

  late final DartExecuteFn aiProvenanceExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_ai_provenance_execute');

  // ── AI Query ────────────────────────────────────────────────────

  late final DartOpenFn aiQueryOpen =
      _lib.lookupFunction<_NativeOpenFn, DartOpenFn>('nodedb_ai_query_open');

  late final DartCloseFn aiQueryClose = _lib
      .lookupFunction<_NativeCloseFn, DartCloseFn>('nodedb_ai_query_close');

  late final DartExecuteFn aiQueryExecute = _lib
      .lookupFunction<_NativeExecuteFn, DartExecuteFn>(
          'nodedb_ai_query_execute');
}
