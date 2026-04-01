use crate::error::TransportError;

/// Trait for handling inbound federated queries.
///
/// The transport layer defines this trait; the FFI layer (or tests) implements it
/// with access to the actual engines (NoSQL, Graph, Vector) and DAC filtering.
/// This inversion of control avoids coupling nodedb-transport to engine crates.
pub trait QueryHandler: Send + Sync {
    /// Handle an inbound federated query.
    ///
    /// - `query_type`: "nosql", "graph", or "vector"
    /// - `query_data`: MessagePack-encoded query payload (same format as FFI execute)
    /// - `origin_peer_id`: the peer who originated the query (for DAC filtering)
    ///
    /// Returns MessagePack-encoded results, or an error.
    fn handle_query(
        &self,
        query_type: &str,
        query_data: &[u8],
        origin_peer_id: &str,
    ) -> Result<Vec<u8>, TransportError>;

    /// Merge a local result with remote results collected from forwarded queries.
    ///
    /// Default implementation: returns local result if non-empty, otherwise the
    /// first non-empty remote result. Override for engine-specific merging
    /// (e.g., NoSQL dedup, Vector sort, Graph union).
    fn merge_results(
        &self,
        _query_type: &str,
        local_result: Vec<u8>,
        remote_results: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        if !local_result.is_empty() {
            return local_result;
        }
        for r in remote_results {
            if !r.is_empty() {
                return r;
            }
        }
        vec![]
    }

    /// Handle an inbound trigger notification from a mesh peer.
    ///
    /// - `payload`: MessagePack-encoded TriggerNotificationPayload
    /// - `origin_peer_id`: the peer that sent the notification
    ///
    /// Default implementation: no-op.
    fn handle_trigger_notification(
        &self,
        _payload: &[u8],
        _origin_peer_id: &str,
    ) {
        // Default: ignore
    }

    /// Handle an inbound preference sync from a mesh peer.
    ///
    /// - `payload`: MessagePack-encoded PreferenceSyncPayload
    /// - `origin_peer_id`: the peer that sent the sync
    ///
    /// Default implementation: no-op.
    fn handle_preference_sync(
        &self,
        _payload: &[u8],
        _origin_peer_id: &str,
    ) {
        // Default: ignore
    }

    /// Handle an inbound singleton sync from a mesh peer.
    ///
    /// - `payload`: MessagePack-encoded SingletonSyncPayload
    /// - `origin_peer_id`: the peer that sent the sync
    ///
    /// Default implementation: no-op.
    fn handle_singleton_sync(
        &self,
        _payload: &[u8],
        _origin_peer_id: &str,
    ) {
        // Default: ignore
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubHandler;
    impl QueryHandler for StubHandler {
        fn handle_query(
            &self,
            _query_type: &str,
            _query_data: &[u8],
            _origin_peer_id: &str,
        ) -> Result<Vec<u8>, TransportError> {
            Ok(vec![])
        }
    }

    #[test]
    fn default_merge_returns_local_if_nonempty() {
        let handler = StubHandler;
        let result = handler.merge_results("nosql", vec![1, 2, 3], vec![vec![4, 5]]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn default_merge_returns_first_remote_if_local_empty() {
        let handler = StubHandler;
        let result = handler.merge_results("nosql", vec![], vec![vec![], vec![7, 8]]);
        assert_eq!(result, vec![7, 8]);
    }

    #[test]
    fn default_merge_returns_empty_if_all_empty() {
        let handler = StubHandler;
        let result = handler.merge_results("nosql", vec![], vec![vec![], vec![]]);
        assert!(result.is_empty());
    }

    #[test]
    fn default_merge_returns_empty_with_no_remotes() {
        let handler = StubHandler;
        let result = handler.merge_results("nosql", vec![], vec![]);
        assert!(result.is_empty());
    }
}
