import '../model/ai_query.dart';

/// Abstract adapter for AI-powered query fallback.
///
/// Implement this class to provide AI-generated data when local and
/// federated queries return no results. The adapter receives the query
/// context (collection schema, query description) and returns candidate
/// records with confidence scores.
///
/// Records above the configured [AiQueryConfig.minimumWriteConfidence]
/// threshold are persisted with provenance tracking. Records below the
/// threshold are returned as unpersisted AI results.
///
/// ```dart
/// class MyAiQueryAdapter extends NodeDbAiQueryAdapter {
///   @override
///   Future<List<AiQueryResult>> queryForMissingData({...}) async {
///     // Call your AI service here
///     return [AiQueryResult(data: {...}, confidence: 0.9, ...)];
///   }
/// }
/// ```
abstract class NodeDbAiQueryAdapter {
  /// Query an AI service for data not found locally or via federation.
  ///
  /// [collection] is the target collection name.
  /// [schemaJson] is the JSON representation of the collection's schema,
  /// including required fields and field types.
  /// [queryDescription] is a natural-language description of the query
  /// derived from the filter AST.
  /// [context] provides configuration like max results and confidence
  /// thresholds.
  ///
  /// Return an empty list if the AI has no relevant data.
  Future<List<AiQueryResult>> queryForMissingData({
    required String collection,
    required String schemaJson,
    required String queryDescription,
    required AiQueryContext context,
  });
}
