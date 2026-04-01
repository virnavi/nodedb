use std::sync::Arc;

use chrono::Utc;
use rmpv::Value;

use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, decode_id, to_msgpack, from_msgpack};

use crate::error::FederationError;
use crate::peer_manager::PeerManager;
use crate::types::NodeGroup;

pub struct GroupManager {
    groups: StorageTree,
    group_names: StorageTree,
    group_members: StorageTree,
    peer_groups: StorageTree,
    id_gen: Arc<IdGenerator>,
}

impl GroupManager {
    pub fn new(engine: &Arc<StorageEngine>, id_gen: Arc<IdGenerator>) -> Result<Self, FederationError> {
        let groups = engine.open_tree("__groups__")?;
        let group_names = engine.open_tree("__group_names__")?;
        let group_members = engine.open_tree("__group_members__")?;
        let peer_groups = engine.open_tree("__peer_groups__")?;
        Ok(GroupManager { groups, group_names, group_members, peer_groups, id_gen })
    }

    fn encode_membership_key(a: i64, b: i64) -> Vec<u8> {
        let mut key = Vec::with_capacity(16);
        key.extend_from_slice(&encode_id(a));
        key.extend_from_slice(&encode_id(b));
        key
    }

    pub fn add_group(
        &self,
        name: &str,
        metadata: Value,
    ) -> Result<NodeGroup, FederationError> {
        if self.group_names.get(name.as_bytes())?.is_some() {
            return Err(FederationError::DuplicateGroupName(name.to_string()));
        }

        let id = self.id_gen.next_id("groups")?;
        let now = Utc::now();
        let group = NodeGroup {
            id,
            name: name.to_string(),
            members: Vec::new(),
            metadata,
            created_at: now,
            updated_at: now,
        };

        let bytes = to_msgpack(&group)?;
        self.groups.insert(&encode_id(id), &bytes)?;
        self.group_names.insert(name.as_bytes(), &encode_id(id))?;

        Ok(group)
    }

    pub fn get_group(&self, id: i64) -> Result<NodeGroup, FederationError> {
        let bytes = self.groups.get(&encode_id(id))?
            .ok_or(FederationError::GroupNotFound(id))?;
        let group: NodeGroup = from_msgpack(&bytes)?;
        Ok(group)
    }

    pub fn get_group_by_name(&self, name: &str) -> Result<NodeGroup, FederationError> {
        let id_bytes = self.group_names.get(name.as_bytes())?
            .ok_or_else(|| FederationError::GroupNotFoundByName(name.to_string()))?;
        let id = decode_id(&id_bytes)?;
        self.get_group(id)
    }

    pub fn update_group(
        &self,
        id: i64,
        metadata: Option<Value>,
    ) -> Result<NodeGroup, FederationError> {
        let mut group = self.get_group(id)?;

        if let Some(m) = metadata {
            group.metadata = m;
        }
        group.updated_at = Utc::now();

        let bytes = to_msgpack(&group)?;
        self.groups.insert(&encode_id(id), &bytes)?;

        Ok(group)
    }

    pub fn delete_group(&self, id: i64) -> Result<NodeGroup, FederationError> {
        let group = self.get_group(id)?;

        // Remove all membership index entries
        for &peer_id in &group.members {
            self.group_members.remove(&Self::encode_membership_key(id, peer_id))?;
            self.peer_groups.remove(&Self::encode_membership_key(peer_id, id))?;
        }

        self.groups.remove(&encode_id(id))?;
        self.group_names.remove(group.name.as_bytes())?;

        Ok(group)
    }

    pub fn add_member(
        &self,
        group_id: i64,
        peer_id: i64,
        peer_mgr: &PeerManager,
    ) -> Result<NodeGroup, FederationError> {
        // Validate peer exists
        if !peer_mgr.peer_exists(peer_id)? {
            return Err(FederationError::InvalidMemberPeer(peer_id));
        }

        let mut group = self.get_group(group_id)?;

        // Skip if already a member
        if group.members.contains(&peer_id) {
            return Ok(group);
        }

        group.members.push(peer_id);
        group.updated_at = Utc::now();

        let bytes = to_msgpack(&group)?;
        self.groups.insert(&encode_id(group_id), &bytes)?;

        // Update both index trees
        self.group_members.insert(&Self::encode_membership_key(group_id, peer_id), &[])?;
        self.peer_groups.insert(&Self::encode_membership_key(peer_id, group_id), &[])?;

        Ok(group)
    }

    pub fn remove_member(
        &self,
        group_id: i64,
        peer_id: i64,
    ) -> Result<NodeGroup, FederationError> {
        let mut group = self.get_group(group_id)?;

        group.members.retain(|&id| id != peer_id);
        group.updated_at = Utc::now();

        let bytes = to_msgpack(&group)?;
        self.groups.insert(&encode_id(group_id), &bytes)?;

        self.group_members.remove(&Self::encode_membership_key(group_id, peer_id))?;
        self.peer_groups.remove(&Self::encode_membership_key(peer_id, group_id))?;

        Ok(group)
    }

    pub fn groups_for_peer(&self, peer_id: i64) -> Result<Vec<i64>, FederationError> {
        let prefix = encode_id(peer_id);
        let mut group_ids = Vec::new();
        for item in self.peer_groups.scan_prefix(&prefix) {
            let (key, _) = item?;
            if key.len() == 16 {
                let gid = decode_id(&key[8..16])?;
                group_ids.push(gid);
            }
        }
        Ok(group_ids)
    }

    /// Remove a peer from all groups they belong to. Used by FederationEngine on delete_peer.
    pub fn remove_peer_from_all_groups(&self, peer_id: i64) -> Result<(), FederationError> {
        let group_ids = self.groups_for_peer(peer_id)?;
        for gid in group_ids {
            self.remove_member(gid, peer_id)?;
        }
        Ok(())
    }

    pub fn all_groups(&self) -> Result<Vec<NodeGroup>, FederationError> {
        let mut groups = Vec::new();
        for item in self.groups.iter() {
            let (_, v) = item?;
            let group: NodeGroup = from_msgpack(&v)?;
            groups.push(group);
        }
        Ok(groups)
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PeerStatus;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<StorageEngine>, Arc<IdGenerator>) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(&dir.path().join("db")).unwrap());
        let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());
        (dir, engine, id_gen)
    }

    #[test]
    fn add_and_get_group() {
        let (_dir, engine, id_gen) = setup();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        let group = grp_mgr.add_group("admins", Value::Nil).unwrap();
        assert_eq!(group.name, "admins");
        assert!(group.members.is_empty());

        let fetched = grp_mgr.get_group(group.id).unwrap();
        assert_eq!(fetched.name, "admins");
    }

    #[test]
    fn get_group_by_name() {
        let (_dir, engine, id_gen) = setup();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        grp_mgr.add_group("editors", Value::Nil).unwrap();
        let group = grp_mgr.get_group_by_name("editors").unwrap();
        assert_eq!(group.name, "editors");
    }

    #[test]
    fn duplicate_group_name_rejected() {
        let (_dir, engine, id_gen) = setup();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        grp_mgr.add_group("admins", Value::Nil).unwrap();
        match grp_mgr.add_group("admins", Value::Nil) {
            Err(FederationError::DuplicateGroupName(name)) => assert_eq!(name, "admins"),
            other => panic!("expected DuplicateGroupName, got {:?}", other),
        }
    }

    #[test]
    fn add_and_remove_members() {
        let (_dir, engine, id_gen) = setup();
        let peer_mgr = PeerManager::new(&engine, id_gen.clone()).unwrap();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        let p1 = peer_mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        let p2 = peer_mgr.add_peer("bob", None, None, PeerStatus::Active, Value::Nil).unwrap();
        let group = grp_mgr.add_group("team", Value::Nil).unwrap();

        // Add members
        let g = grp_mgr.add_member(group.id, p1.id, &peer_mgr).unwrap();
        assert_eq!(g.members, vec![p1.id]);

        let g = grp_mgr.add_member(group.id, p2.id, &peer_mgr).unwrap();
        assert_eq!(g.members, vec![p1.id, p2.id]);

        // Idempotent add
        let g = grp_mgr.add_member(group.id, p1.id, &peer_mgr).unwrap();
        assert_eq!(g.members, vec![p1.id, p2.id]);

        // Remove member
        let g = grp_mgr.remove_member(group.id, p1.id).unwrap();
        assert_eq!(g.members, vec![p2.id]);
    }

    #[test]
    fn invalid_peer_rejected() {
        let (_dir, engine, id_gen) = setup();
        let peer_mgr = PeerManager::new(&engine, id_gen.clone()).unwrap();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        let group = grp_mgr.add_group("team", Value::Nil).unwrap();
        match grp_mgr.add_member(group.id, 999, &peer_mgr) {
            Err(FederationError::InvalidMemberPeer(999)) => {}
            other => panic!("expected InvalidMemberPeer, got {:?}", other),
        }
    }

    #[test]
    fn groups_for_peer() {
        let (_dir, engine, id_gen) = setup();
        let peer_mgr = PeerManager::new(&engine, id_gen.clone()).unwrap();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        let p1 = peer_mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        let g1 = grp_mgr.add_group("admins", Value::Nil).unwrap();
        let g2 = grp_mgr.add_group("editors", Value::Nil).unwrap();

        grp_mgr.add_member(g1.id, p1.id, &peer_mgr).unwrap();
        grp_mgr.add_member(g2.id, p1.id, &peer_mgr).unwrap();

        let mut gids = grp_mgr.groups_for_peer(p1.id).unwrap();
        gids.sort();
        assert_eq!(gids, vec![g1.id, g2.id]);
    }

    #[test]
    fn delete_group_cleans_indexes() {
        let (_dir, engine, id_gen) = setup();
        let peer_mgr = PeerManager::new(&engine, id_gen.clone()).unwrap();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        let p1 = peer_mgr.add_peer("alice", None, None, PeerStatus::Active, Value::Nil).unwrap();
        let group = grp_mgr.add_group("team", Value::Nil).unwrap();
        grp_mgr.add_member(group.id, p1.id, &peer_mgr).unwrap();

        grp_mgr.delete_group(group.id).unwrap();
        assert_eq!(grp_mgr.group_count(), 0);

        // Peer should no longer be in any groups
        let gids = grp_mgr.groups_for_peer(p1.id).unwrap();
        assert!(gids.is_empty());
    }

    #[test]
    fn all_groups_and_count() {
        let (_dir, engine, id_gen) = setup();
        let grp_mgr = GroupManager::new(&engine, id_gen).unwrap();

        grp_mgr.add_group("a", Value::Nil).unwrap();
        grp_mgr.add_group("b", Value::Nil).unwrap();

        assert_eq!(grp_mgr.group_count(), 2);
        assert_eq!(grp_mgr.all_groups().unwrap().len(), 2);
    }
}
