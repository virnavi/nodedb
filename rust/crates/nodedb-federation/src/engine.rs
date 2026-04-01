use std::sync::Arc;

use rmpv::Value;

use nodedb_storage::{StorageEngine, IdGenerator};

use crate::error::FederationError;
use crate::peer_manager::PeerManager;
use crate::group_manager::GroupManager;
use crate::types::{NodePeer, NodeGroup, PeerStatus};

pub struct FederationEngine {
    engine: Arc<StorageEngine>,
    peer_mgr: PeerManager,
    group_mgr: GroupManager,
}

impl FederationEngine {
    pub fn new(engine: Arc<StorageEngine>) -> Result<Self, FederationError> {
        let id_gen = Arc::new(IdGenerator::new(&engine)?);
        let peer_mgr = PeerManager::new(&engine, id_gen.clone())?;
        let group_mgr = GroupManager::new(&engine, id_gen)?;
        Ok(FederationEngine { engine, peer_mgr, group_mgr })
    }

    // --- Peer operations ---

    pub fn add_peer(
        &self,
        name: &str,
        endpoint: Option<String>,
        public_key: Option<String>,
        status: PeerStatus,
        metadata: Value,
    ) -> Result<NodePeer, FederationError> {
        self.peer_mgr.add_peer(name, endpoint, public_key, status, metadata)
    }

    pub fn get_peer(&self, id: i64) -> Result<NodePeer, FederationError> {
        self.peer_mgr.get_peer(id)
    }

    pub fn get_peer_by_name(&self, name: &str) -> Result<NodePeer, FederationError> {
        self.peer_mgr.get_peer_by_name(name)
    }

    pub fn update_peer(
        &self,
        id: i64,
        endpoint: Option<Option<String>>,
        public_key: Option<Option<String>>,
        status: Option<PeerStatus>,
        metadata: Option<Value>,
    ) -> Result<NodePeer, FederationError> {
        self.peer_mgr.update_peer(id, endpoint, public_key, status, metadata)
    }

    /// Delete a peer and remove it from all groups (cascade).
    pub fn delete_peer(&self, id: i64) -> Result<NodePeer, FederationError> {
        // Remove from all group memberships first
        self.group_mgr.remove_peer_from_all_groups(id)?;
        self.peer_mgr.delete_peer(id)
    }

    pub fn all_peers(&self) -> Result<Vec<NodePeer>, FederationError> {
        self.peer_mgr.all_peers()
    }

    pub fn peer_count(&self) -> usize {
        self.peer_mgr.peer_count()
    }

    // --- Group operations ---

    pub fn add_group(&self, name: &str, metadata: Value) -> Result<NodeGroup, FederationError> {
        self.group_mgr.add_group(name, metadata)
    }

    pub fn get_group(&self, id: i64) -> Result<NodeGroup, FederationError> {
        self.group_mgr.get_group(id)
    }

    pub fn get_group_by_name(&self, name: &str) -> Result<NodeGroup, FederationError> {
        self.group_mgr.get_group_by_name(name)
    }

    pub fn update_group(&self, id: i64, metadata: Option<Value>) -> Result<NodeGroup, FederationError> {
        self.group_mgr.update_group(id, metadata)
    }

    pub fn delete_group(&self, id: i64) -> Result<NodeGroup, FederationError> {
        self.group_mgr.delete_group(id)
    }

    pub fn add_member(&self, group_id: i64, peer_id: i64) -> Result<NodeGroup, FederationError> {
        self.group_mgr.add_member(group_id, peer_id, &self.peer_mgr)
    }

    pub fn remove_member(&self, group_id: i64, peer_id: i64) -> Result<NodeGroup, FederationError> {
        self.group_mgr.remove_member(group_id, peer_id)
    }

    pub fn groups_for_peer(&self, peer_id: i64) -> Result<Vec<i64>, FederationError> {
        self.group_mgr.groups_for_peer(peer_id)
    }

    pub fn all_groups(&self) -> Result<Vec<NodeGroup>, FederationError> {
        self.group_mgr.all_groups()
    }

    pub fn group_count(&self) -> usize {
        self.group_mgr.group_count()
    }

    pub fn flush(&self) -> Result<(), FederationError> {
        self.engine.flush()?;
        Ok(())
    }
}
