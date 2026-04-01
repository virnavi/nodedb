import 'package:nodedb/nodedb.dart';

/// Convert a [Document] to a JSON-serializable map.
Map<String, dynamic> documentToJson(Document doc) => {
      'id': doc.id,
      'collection': doc.collection,
      'data': doc.data,
      'createdAt': doc.createdAt.toIso8601String(),
      'updatedAt': doc.updatedAt.toIso8601String(),
    };

/// Convert a [GraphNode] to a JSON-serializable map.
Map<String, dynamic> graphNodeToJson(GraphNode node) => {
      'id': node.id,
      'label': node.label,
      'data': node.data,
    };

/// Convert a [GraphEdge] to a JSON-serializable map.
Map<String, dynamic> graphEdgeToJson(GraphEdge edge) => {
      'id': edge.id,
      'label': edge.label,
      'source': edge.source,
      'target': edge.target,
      'weight': edge.weight,
      'data': edge.data,
    };

/// Convert a [NodePeer] to a JSON-serializable map.
Map<String, dynamic> nodePeerToJson(NodePeer peer) => {
      'id': peer.id,
      'name': peer.name,
      'endpoint': peer.endpoint,
      'status': peer.status,
      if (peer.metadata != null) 'metadata': peer.metadata,
    };

/// Convert a [NodeGroup] to a JSON-serializable map.
Map<String, dynamic> nodeGroupToJson(NodeGroup group) => {
      'id': group.id,
      'name': group.name,
      'members': group.members,
      if (group.metadata != null) 'metadata': group.metadata,
    };

/// Convert an [AccessRule] to a JSON-serializable map.
Map<String, dynamic> accessRuleToJson(AccessRule rule) => {
      'id': rule.id,
      'collection': rule.collection,
      if (rule.field != null) 'field': rule.field,
      if (rule.recordId != null) 'recordId': rule.recordId,
      'subjectType': rule.subjectType,
      'subjectId': rule.subjectId,
      'permission': rule.permission,
      if (rule.expiresAt != null) 'expiresAt': rule.expiresAt!.toIso8601String(),
    };

/// Convert a [ProvenanceEnvelope] to a JSON-serializable map.
Map<String, dynamic> provenanceEnvelopeToJson(ProvenanceEnvelope env) => {
      'id': env.id,
      'collection': env.collection,
      'recordId': env.recordId,
      'confidenceFactor': env.confidenceFactor,
      'sourceId': env.sourceId,
      'sourceType': env.sourceType,
      'contentHash': env.contentHash,
      'createdAtUtc': env.createdAtUtc.toIso8601String(),
      'updatedAtUtc': env.updatedAtUtc.toIso8601String(),
      if (env.pkiSignature != null) 'pkiSignature': env.pkiSignature,
      if (env.pkiId != null) 'pkiId': env.pkiId,
      if (env.userId != null) 'userId': env.userId,
      'verificationStatus': env.verificationStatus,
      'aiAugmented': env.aiAugmented,
      if (env.aiRawConfidence != null) 'aiRawConfidence': env.aiRawConfidence,
      if (env.aiBlendWeightUsed != null)
        'aiBlendWeightUsed': env.aiBlendWeightUsed,
      if (env.aiReasoning != null) 'aiReasoning': env.aiReasoning,
      if (env.aiTags != null) 'aiTags': env.aiTags,
      'aiAnomalyFlagged': env.aiAnomalyFlagged,
      if (env.aiAnomalySeverity != null)
        'aiAnomalySeverity': env.aiAnomalySeverity,
      'aiOriginated': env.aiOriginated,
      if (env.aiOriginTag != null) 'aiOriginTag': env.aiOriginTag,
      if (env.aiSourceExplanation != null)
        'aiSourceExplanation': env.aiSourceExplanation,
      if (env.aiExternalSourceUri != null)
        'aiExternalSourceUri': env.aiExternalSourceUri,
      if (env.checkedAtUtc != null)
        'checkedAtUtc': env.checkedAtUtc!.toIso8601String(),
      if (env.dataUpdatedAtUtc != null)
        'dataUpdatedAtUtc': env.dataUpdatedAtUtc!.toIso8601String(),
      if (env.localId != null) 'localId': env.localId,
      if (env.globalId != null) 'globalId': env.globalId,
    };

/// Convert a [KeyEntry] to a JSON-serializable map.
Map<String, dynamic> keyEntryToJson(KeyEntry entry) => {
      'id': entry.id,
      'pkiId': entry.pkiId,
      'userId': entry.userId,
      'publicKeyHex': entry.publicKeyHex,
      'trustLevel': entry.trustLevel,
      if (entry.expiresAtUtc != null)
        'expiresAtUtc': entry.expiresAtUtc!.toIso8601String(),
      'createdAtUtc': entry.createdAtUtc.toIso8601String(),
    };

/// Convert a [SearchResult] to a JSON-serializable map.
Map<String, dynamic> searchResultToJson(SearchResult result) => {
      'id': result.id,
      'distance': result.distance,
      'metadata': result.metadata,
    };

/// Convert a [VectorRecord] to a JSON-serializable map.
Map<String, dynamic> vectorRecordToJson(VectorRecord record) => {
      'id': record.id,
      'vector': record.vector,
      'metadata': record.metadata,
    };
