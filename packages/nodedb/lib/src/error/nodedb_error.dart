import 'error_codes.dart';

/// Base exception for all NodeDB errors.
class NodeDbException implements Exception {
  final int code;
  final String message;

  const NodeDbException(this.code, this.message);

  /// Create a typed exception from an FFI error code.
  factory NodeDbException.fromCode(int code, String message) {
    if (code == errInvalidHandle) return InvalidHandleException(message);
    if (code == errStorage) return StorageException(message);
    if (code == errSerialization) return SerializationException(message);
    if (code == errNotFound) return NotFoundException(message);
    if (code == errInvalidQuery) return InvalidQueryException(message);
    if (code == errNullPointer) return NullPointerException(message);

    // Graph errors
    if (code >= 10 && code <= 13) return GraphException(code, message);

    // Vector errors
    if (code >= 20 && code <= 23) return VectorException(code, message);

    // Federation errors
    if (code >= 30 && code <= 33) return FederationException(code, message);

    // DAC errors
    if (code >= 40 && code <= 42) return DacException(code, message);

    // Transport errors
    if (code >= 50 && code <= 55) return TransportException(code, message);

    // Provenance errors
    if (code >= 60 && code <= 63) return ProvenanceException(code, message);

    // Key Resolver errors
    if (code >= 70 && code <= 73) return KeyResolverException(code, message);

    // AI Provenance errors
    if (code >= 80 && code <= 83) {
      return AiProvenanceException(code, message);
    }

    // AI Query errors
    if (code >= 90 && code <= 94) return AiQueryException(code, message);

    // Trigger errors
    if (code == errTriggerAbort) return TriggerAbortException(message);
    if (code == errTriggerNotFound) return TriggerNotFoundException(message);

    // Singleton errors
    if (code == errSingletonDelete) return SingletonDeleteException(message);
    if (code == errSingletonClear) return SingletonClearException(message);

    // Preference errors
    if (code == errPreferenceNotFound) {
      return PreferenceNotFoundException(message);
    }
    if (code == errPreferenceError) return PreferenceException(message);

    // Reserved schema errors
    if (code == errReservedSchemaWrite) {
      return ReservedSchemaWriteException(message);
    }

    return NodeDbException(code, message);
  }

  @override
  String toString() => 'NodeDbException($code): $message';
}

// ignore: use_super_parameters — positional super params can't mix with explicit super()
class InvalidHandleException extends NodeDbException {
  const InvalidHandleException(String message)
      : super(errInvalidHandle, message);
}

class StorageException extends NodeDbException {
  const StorageException(String message) : super(errStorage, message);
}

class SerializationException extends NodeDbException {
  const SerializationException(String message)
      : super(errSerialization, message);
}

class NotFoundException extends NodeDbException {
  const NotFoundException(String message) : super(errNotFound, message);
}

class InvalidQueryException extends NodeDbException {
  const InvalidQueryException(String message)
      : super(errInvalidQuery, message);
}

class NullPointerException extends NodeDbException {
  const NullPointerException(String message) : super(errNullPointer, message);
}

class GraphException extends NodeDbException {
  const GraphException(super.code, super.message);
}

class VectorException extends NodeDbException {
  const VectorException(super.code, super.message);
}

class FederationException extends NodeDbException {
  const FederationException(super.code, super.message);
}

class DacException extends NodeDbException {
  const DacException(super.code, super.message);
}

class TransportException extends NodeDbException {
  const TransportException(super.code, super.message);
}

class ProvenanceException extends NodeDbException {
  const ProvenanceException(super.code, super.message);
}

class KeyResolverException extends NodeDbException {
  const KeyResolverException(super.code, super.message);
}

class AiProvenanceException extends NodeDbException {
  const AiProvenanceException(super.code, super.message);
}

class AiQueryException extends NodeDbException {
  const AiQueryException(super.code, super.message);
}

class TriggerAbortException extends NodeDbException {
  const TriggerAbortException(String message)
      : super(errTriggerAbort, message);
}

class TriggerNotFoundException extends NodeDbException {
  const TriggerNotFoundException(String message)
      : super(errTriggerNotFound, message);
}

class SingletonDeleteException extends NodeDbException {
  const SingletonDeleteException(String message)
      : super(errSingletonDelete, message);
}

class SingletonClearException extends NodeDbException {
  const SingletonClearException(String message)
      : super(errSingletonClear, message);
}

class PreferenceNotFoundException extends NodeDbException {
  const PreferenceNotFoundException(String message)
      : super(errPreferenceNotFound, message);
}

class PreferenceException extends NodeDbException {
  const PreferenceException(String message)
      : super(errPreferenceError, message);
}

class ReservedSchemaWriteException extends NodeDbException {
  const ReservedSchemaWriteException(String message)
      : super(errReservedSchemaWrite, message);
}
