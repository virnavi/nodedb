use std::sync::Arc;

use dashmap::DashMap;
use nodedb_crypto::PublicIdentity;

use crate::types::{PeerAcceptance, PeerCredential};

/// In-memory credential store. Credentials are NEVER persisted to disk.
pub struct CredentialStore {
    credentials: DashMap<String, PeerCredential>,
    acceptance_callback: Arc<dyn Fn(&PublicIdentity) -> PeerAcceptance + Send + Sync>,
}

impl CredentialStore {
    /// Create with a custom acceptance callback.
    pub fn new(
        callback: impl Fn(&PublicIdentity) -> PeerAcceptance + Send + Sync + 'static,
    ) -> Self {
        CredentialStore {
            credentials: DashMap::new(),
            acceptance_callback: Arc::new(callback),
        }
    }

    /// Create with default accept-all policy.
    pub fn accept_all() -> Self {
        Self::new(|_| PeerAcceptance::Accept)
    }

    pub fn set_credential(&self, peer_id: &str, cred: PeerCredential) {
        self.credentials.insert(peer_id.to_string(), cred);
    }

    pub fn get_credential(&self, peer_id: &str) -> Option<PeerCredential> {
        self.credentials.get(peer_id).map(|r| r.value().clone())
    }

    pub fn remove_credential(&self, peer_id: &str) {
        self.credentials.remove(peer_id);
    }

    pub fn should_accept(&self, identity: &PublicIdentity) -> PeerAcceptance {
        (self.acceptance_callback)(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_all_by_default() {
        let store = CredentialStore::accept_all();
        let pi = PublicIdentity {
            peer_id: "test".to_string(),
            public_key_bytes: vec![0u8; 32],
        };
        assert_eq!(store.should_accept(&pi), PeerAcceptance::Accept);
    }

    #[test]
    fn custom_rejection() {
        let store = CredentialStore::new(|pi| {
            if pi.peer_id == "banned" {
                PeerAcceptance::Reject
            } else {
                PeerAcceptance::Accept
            }
        });
        let ok = PublicIdentity {
            peer_id: "good".to_string(),
            public_key_bytes: vec![0u8; 32],
        };
        let bad = PublicIdentity {
            peer_id: "banned".to_string(),
            public_key_bytes: vec![0u8; 32],
        };
        assert_eq!(store.should_accept(&ok), PeerAcceptance::Accept);
        assert_eq!(store.should_accept(&bad), PeerAcceptance::Reject);
    }

    #[test]
    fn credential_crud() {
        let store = CredentialStore::accept_all();
        assert!(store.get_credential("peer1").is_none());

        store.set_credential("peer1", PeerCredential::BearerToken("token123".into()));
        assert!(store.get_credential("peer1").is_some());

        store.remove_credential("peer1");
        assert!(store.get_credential("peer1").is_none());
    }
}
