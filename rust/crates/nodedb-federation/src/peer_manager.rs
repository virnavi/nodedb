use std::sync::Arc;

use chrono::Utc;
use rmpv::Value;

use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, decode_id, to_msgpack, from_msgpack};

use crate::error::FederationError;
use crate::types::{NodePeer, PeerStatus};

pub struct PeerManager {
    peers: StorageTree,
    peer_names: StorageTree,
    id_gen: Arc<IdGenerator>,
}

impl PeerManager {
    pub fn new(engine: &Arc<StorageEngine>, id_gen: Arc<IdGenerator>) -> Result<Self, FederationError> {
        let peers = engine.open_tree("__peers__")?;
        let peer_names = engine.open_tree("__peer_names__")?;
        Ok(PeerManager { peers, peer_names, id_gen })
    }

    pub fn add_peer(
        &self,
        name: &str,
        endpoint: Option<String>,
        public_key: Option<String>,
        status: PeerStatus,
        metadata: Value,
    ) -> Result<NodePeer, FederationError> {
        // Check name uniqueness
        if self.peer_names.get(name.as_bytes())?.is_some() {
            return Err(FederationError::DuplicatePeerName(name.to_string()));
        }

        let id = self.id_gen.next_id("peers")?;
        let now = Utc::now();
        let peer = NodePeer {
            id,
            name: name.to_string(),
            endpoint,
            public_key,
            status,
            metadata,
            created_at: now,
            updated_at: now,
        };

        let bytes = to_msgpack(&peer)?;
        self.peers.insert(&encode_id(id), &bytes)?;
        self.peer_names.insert(name.as_bytes(), &encode_id(id))?;

        Ok(peer)
    }

    pub fn get_peer(&self, id: i64) -> Result<NodePeer, FederationError> {
        let bytes = self.peers.get(&encode_id(id))?
            .ok_or(FederationError::PeerNotFound(id))?;
        let peer: NodePeer = from_msgpack(&bytes)?;
        Ok(peer)
    }

    pub fn get_peer_by_name(&self, name: &str) -> Result<NodePeer, FederationError> {
        let id_bytes = self.peer_names.get(name.as_bytes())?
            .ok_or_else(|| FederationError::PeerNotFoundByName(name.to_string()))?;
        let id = decode_id(&id_bytes)?;
        self.get_peer(id)
    }

    pub fn update_peer(
        &self,
        id: i64,
        endpoint: Option<Option<String>>,
        public_key: Option<Option<String>>,
        status: Option<PeerStatus>,
        metadata: Option<Value>,
    ) -> Result<NodePeer, FederationError> {
        let mut peer = self.get_peer(id)?;

        if let Some(ep) = endpoint {
            peer.endpoint = ep;
        }
        if let Some(pk) = public_key {
            peer.public_key = pk;
        }
        if let Some(s) = status {
            peer.status = s;
        }
        if let Some(m) = metadata {
            peer.metadata = m;
        }
        peer.updated_at = Utc::now();

        let bytes = to_msgpack(&peer)?;
        self.peers.insert(&encode_id(id), &bytes)?;

        Ok(peer)
    }

    pub fn delete_peer(&self, id: i64) -> Result<NodePeer, FederationError> {
        let peer = self.get_peer(id)?;
        self.peers.remove(&encode_id(id))?;
        self.peer_names.remove(peer.name.as_bytes())?;
        Ok(peer)
    }

    pub fn all_peers(&self) -> Result<Vec<NodePeer>, FederationError> {
        let mut peers = Vec::new();
        for item in self.peers.iter() {
            let (_, v) = item?;
            let peer: NodePeer = from_msgpack(&v)?;
            peers.push(peer);
        }
        Ok(peers)
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn peer_exists(&self, id: i64) -> Result<bool, FederationError> {
        Ok(self.peers.get(&encode_id(id))?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<StorageEngine>, Arc<IdGenerator>) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
        let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());
        (dir, engine, id_gen)
    }

    #[test]
    fn add_and_get_peer() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        let peer = mgr.add_peer("alice", Some("ws://localhost:8080".into()), None, PeerStatus::Active, Value::Nil).unwrap();
        assert_eq!(peer.id, 1);
        assert_eq!(peer.name, "alice");
        assert_eq!(peer.status, PeerStatus::Active);

        let fetched = mgr.get_peer(1).unwrap();
        assert_eq!(fetched.name, "alice");
    }

    #[test]
    fn get_peer_by_name() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        mgr.add_peer("bob", None, None, PeerStatus::Inactive, Value::Nil).unwrap();
        let peer = mgr.get_peer_by_name("bob").unwrap();
        assert_eq!(peer.name, "bob");
        assert_eq!(peer.status, PeerStatus::Inactive);
    }

    #[test]
    fn duplicate_name_rejected() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        match mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil) {
            Err(FederationError::DuplicatePeerName(name)) => assert_eq!(name, "alice"),
            other => panic!("expected DuplicatePeerName, got {:?}", other),
        }
    }

    #[test]
    fn update_peer() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        let updated = mgr.update_peer(1, None, None, Some(PeerStatus::Banned), None).unwrap();
        assert_eq!(updated.status, PeerStatus::Banned);

        let fetched = mgr.get_peer(1).unwrap();
        assert_eq!(fetched.status, PeerStatus::Banned);
    }

    #[test]
    fn delete_peer() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        assert_eq!(mgr.peer_count(), 1);

        let deleted = mgr.delete_peer(1).unwrap();
        assert_eq!(deleted.name, "alice");
        assert_eq!(mgr.peer_count(), 0);

        // Name freed for reuse
        match mgr.get_peer_by_name("alice") {
            Err(FederationError::PeerNotFoundByName(_)) => {}
            other => panic!("expected PeerNotFoundByName, got {:?}", other),
        }
    }

    #[test]
    fn all_peers_and_count() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        mgr.add_peer("bob", None, None, PeerStatus::Inactive, Value::Nil).unwrap();
        mgr.add_peer("carol", None, None, PeerStatus::Banned, Value::Nil).unwrap();

        assert_eq!(mgr.peer_count(), 3);
        let all = mgr.all_peers().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn peer_not_found() {
        let (_dir, engine, id_gen) = setup();
        let mgr = PeerManager::new(&engine, id_gen).unwrap();

        match mgr.get_peer(999) {
            Err(FederationError::PeerNotFound(999)) => {}
            other => panic!("expected PeerNotFound, got {:?}", other),
        }
    }
}
