import '../model/provenance_envelope.dart';

/// Wraps a result with its provenance envelope.
class WithProvenance<T> {
  final T data;
  final ProvenanceEnvelope? provenance;

  const WithProvenance(this.data, this.provenance);

  @override
  String toString() => 'WithProvenance($data, provenance: $provenance)';
}

/// Wraps a result from a federated query with the source peer ID.
class FederatedResult<T> {
  final T data;
  final String sourcePeerId;

  const FederatedResult(this.data, this.sourcePeerId);

  @override
  String toString() => 'FederatedResult($data, peer: $sourcePeerId)';
}
