use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use nodedb_storage::{StorageEngine, StorageTree, IdGenerator, encode_id, decode_id, to_msgpack, from_msgpack};

use crate::error::KeyResolverError;
use crate::types::{NodePublicKeyEntry, KeyTrustLevel, KeyResolutionResult};

pub struct KeyResolverEngine {
    #[allow(dead_code)]
    engine: Arc<StorageEngine>,
    keys: StorageTree,
    by_pki_user: StorageTree,
    id_gen: Arc<IdGenerator>,
    trust_all_global: AtomicBool,
    trust_all_peers: RwLock<HashSet<String>>,
}

fn make_pki_user_key(pki_id: &str, user_id: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(pki_id.len() + 1 + user_id.len());
    key.extend_from_slice(pki_id.as_bytes());
    key.push(0x00);
    key.extend_from_slice(user_id.as_bytes());
    key
}

fn validate_hex(hex: &str) -> Result<(), KeyResolverError> {
    if hex.len() != 64 {
        return Err(KeyResolverError::InvalidPublicKeyHex(
            format!("expected 64 hex chars, got {}", hex.len()),
        ));
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(KeyResolverError::InvalidPublicKeyHex(
            "contains non-hex characters".to_string(),
        ));
    }
    Ok(())
}

impl KeyResolverEngine {
    pub fn new(engine: Arc<StorageEngine>, id_gen: Arc<IdGenerator>) -> Result<Self, KeyResolverError> {
        let keys = engine.open_tree("__key_cache__")?;
        let by_pki_user = engine.open_tree("__key_cache_by_pki_user__")?;
        Ok(KeyResolverEngine {
            engine,
            keys,
            by_pki_user,
            id_gen,
            trust_all_global: AtomicBool::new(false),
            trust_all_peers: RwLock::new(HashSet::new()),
        })
    }

    pub fn supply_key(
        &self,
        pki_id: &str,
        user_id: &str,
        public_key_hex: &str,
        trust_level: KeyTrustLevel,
        expires_at_utc: Option<String>,
    ) -> Result<NodePublicKeyEntry, KeyResolverError> {
        validate_hex(public_key_hex)?;

        let composite = make_pki_user_key(pki_id, user_id);
        let now = Utc::now().to_rfc3339();

        // Check if entry already exists (upsert)
        if let Some(id_bytes) = self.by_pki_user.get(&composite)? {
            let id = decode_id(&id_bytes)?;
            let entry = NodePublicKeyEntry {
                id,
                pki_id: pki_id.to_string(),
                user_id: user_id.to_string(),
                public_key_hex: public_key_hex.to_string(),
                trust_level,
                cached_at_utc: now,
                expires_at_utc,
            };
            let bytes = to_msgpack(&entry)?;
            self.keys.insert(&encode_id(id), &bytes)?;
            return Ok(entry);
        }

        let id = self.id_gen.next_id("keyresolver")?;
        let entry = NodePublicKeyEntry {
            id,
            pki_id: pki_id.to_string(),
            user_id: user_id.to_string(),
            public_key_hex: public_key_hex.to_string(),
            trust_level,
            cached_at_utc: now,
            expires_at_utc,
        };
        let bytes = to_msgpack(&entry)?;
        self.keys.insert(&encode_id(id), &bytes)?;
        self.by_pki_user.insert(&composite, &encode_id(id))?;
        Ok(entry)
    }

    pub fn key_count(&self) -> Result<usize, KeyResolverError> {
        Ok(self.keys.len())
    }

    pub fn get_key(&self, pki_id: &str, user_id: &str) -> Result<NodePublicKeyEntry, KeyResolverError> {
        let composite = make_pki_user_key(pki_id, user_id);
        let id_bytes = self.by_pki_user.get(&composite)?
            .ok_or_else(|| KeyResolverError::KeyNotFound(pki_id.to_string(), user_id.to_string()))?;
        let id = decode_id(&id_bytes)?;
        self.get_key_by_id(id)
    }

    pub fn get_key_by_id(&self, id: i64) -> Result<NodePublicKeyEntry, KeyResolverError> {
        let bytes = self.keys.get(&encode_id(id))?
            .ok_or(KeyResolverError::EntryNotFound(id))?;
        let entry: NodePublicKeyEntry = from_msgpack(&bytes)?;
        Ok(entry)
    }

    pub fn all_keys(&self) -> Result<Vec<NodePublicKeyEntry>, KeyResolverError> {
        let mut result = Vec::new();
        for item in self.keys.iter() {
            let (_, v) = item?;
            let entry: NodePublicKeyEntry = from_msgpack(&v)?;
            result.push(entry);
        }
        Ok(result)
    }

    pub fn revoke_key(&self, pki_id: &str, user_id: &str) -> Result<NodePublicKeyEntry, KeyResolverError> {
        let composite = make_pki_user_key(pki_id, user_id);
        let id_bytes = self.by_pki_user.get(&composite)?
            .ok_or_else(|| KeyResolverError::KeyNotFound(pki_id.to_string(), user_id.to_string()))?;
        let id = decode_id(&id_bytes)?;
        let mut entry: NodePublicKeyEntry = from_msgpack(
            &self.keys.get(&encode_id(id))?.ok_or(KeyResolverError::EntryNotFound(id))?,
        )?;
        entry.trust_level = KeyTrustLevel::Revoked;
        let bytes = to_msgpack(&entry)?;
        self.keys.insert(&encode_id(id), &bytes)?;
        Ok(entry)
    }

    pub fn delete_key(&self, id: i64) -> Result<(), KeyResolverError> {
        let bytes = self.keys.get(&encode_id(id))?
            .ok_or(KeyResolverError::EntryNotFound(id))?;
        let entry: NodePublicKeyEntry = from_msgpack(&bytes)?;
        let composite = make_pki_user_key(&entry.pki_id, &entry.user_id);
        self.keys.remove(&encode_id(id))?;
        self.by_pki_user.remove(&composite)?;
        Ok(())
    }

    pub fn set_trust_all(&self, enabled: bool) {
        self.trust_all_global.store(enabled, Ordering::SeqCst);
    }

    pub fn set_trust_all_for_peer(&self, peer_id: &str, enabled: bool) {
        let mut peers = self.trust_all_peers.write().unwrap();
        if enabled {
            peers.insert(peer_id.to_string());
        } else {
            peers.remove(peer_id);
        }
    }

    pub fn is_trust_all_active(&self) -> bool {
        self.trust_all_global.load(Ordering::SeqCst)
    }

    pub fn is_trust_all_for_peer(&self, peer_id: &str) -> bool {
        if self.trust_all_global.load(Ordering::SeqCst) {
            return true;
        }
        let peers = self.trust_all_peers.read().unwrap();
        peers.contains(peer_id)
    }

    pub fn resolve_for_verification(
        &self,
        pki_id: &str,
        user_id: &str,
    ) -> Result<KeyResolutionResult, KeyResolverError> {
        let composite = make_pki_user_key(pki_id, user_id);
        let id_bytes = match self.by_pki_user.get(&composite)? {
            Some(b) => b,
            None => return Ok(KeyResolutionResult::NotFound),
        };
        let id = decode_id(&id_bytes)?;
        let bytes = match self.keys.get(&encode_id(id))? {
            Some(b) => b,
            None => return Ok(KeyResolutionResult::NotFound),
        };
        let entry: NodePublicKeyEntry = from_msgpack(&bytes)?;

        // Check expiry
        if let Some(ref expires) = entry.expires_at_utc {
            if let Ok(exp) = chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(expires) {
                if exp < Utc::now() {
                    return Ok(KeyResolutionResult::Expired);
                }
            }
        }

        Ok(KeyResolutionResult::Found(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Arc<StorageEngine>, Arc<IdGenerator>, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = Arc::new(StorageEngine::open(dir.path()).unwrap());
        let id_gen = Arc::new(IdGenerator::new(&engine).unwrap());
        (engine, id_gen, dir)
    }

    fn valid_hex() -> String {
        "ab".repeat(32)
    }

    #[test]
    fn new_engine() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert_eq!(kr.key_count().unwrap(), 0);
    }

    #[test]
    fn supply_and_get_key() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        let entry = kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        assert_eq!(entry.pki_id, "pki1");
        assert_eq!(entry.user_id, "user1");
        assert_eq!(entry.trust_level, KeyTrustLevel::Explicit);
        assert_eq!(kr.key_count().unwrap(), 1);

        let fetched = kr.get_key("pki1", "user1").unwrap();
        assert_eq!(fetched.id, entry.id);
        assert_eq!(fetched.public_key_hex, valid_hex());
    }

    #[test]
    fn supply_upsert() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        let e1 = kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        let new_hex = "cd".repeat(32);
        let e2 = kr.supply_key("pki1", "user1", &new_hex, KeyTrustLevel::TrustAll, None).unwrap();
        assert_eq!(e1.id, e2.id); // same ID (upsert)
        assert_eq!(e2.public_key_hex, new_hex);
        assert_eq!(e2.trust_level, KeyTrustLevel::TrustAll);
        assert_eq!(kr.key_count().unwrap(), 1);
    }

    #[test]
    fn supply_invalid_hex_rejected() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(kr.supply_key("pki1", "user1", "tooshort", KeyTrustLevel::Explicit, None).is_err());
        assert!(kr.supply_key("pki1", "user1", &"zz".repeat(32), KeyTrustLevel::Explicit, None).is_err());
    }

    #[test]
    fn get_key_not_found() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(kr.get_key("missing", "user").is_err());
    }

    #[test]
    fn get_key_by_id() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        let entry = kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        let fetched = kr.get_key_by_id(entry.id).unwrap();
        assert_eq!(fetched.pki_id, "pki1");
        assert!(kr.get_key_by_id(999).is_err());
    }

    #[test]
    fn all_keys() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(kr.all_keys().unwrap().is_empty());
        kr.supply_key("pki1", "u1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        kr.supply_key("pki2", "u2", &"cd".repeat(32), KeyTrustLevel::Explicit, None).unwrap();
        assert_eq!(kr.all_keys().unwrap().len(), 2);
    }

    #[test]
    fn revoke_key() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        let revoked = kr.revoke_key("pki1", "user1").unwrap();
        assert_eq!(revoked.trust_level, KeyTrustLevel::Revoked);
        let fetched = kr.get_key("pki1", "user1").unwrap();
        assert_eq!(fetched.trust_level, KeyTrustLevel::Revoked);
    }

    #[test]
    fn revoke_nonexistent() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(kr.revoke_key("missing", "user").is_err());
    }

    #[test]
    fn delete_key() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        let entry = kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        kr.delete_key(entry.id).unwrap();
        assert_eq!(kr.key_count().unwrap(), 0);
        assert!(kr.get_key("pki1", "user1").is_err());
    }

    #[test]
    fn delete_nonexistent() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(kr.delete_key(999).is_err());
    }

    #[test]
    fn trust_all_default_off() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        assert!(!kr.is_trust_all_active());
        assert!(!kr.is_trust_all_for_peer("peer1"));
    }

    #[test]
    fn trust_all_global_toggle() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.set_trust_all(true);
        assert!(kr.is_trust_all_active());
        assert!(kr.is_trust_all_for_peer("any-peer"));
        kr.set_trust_all(false);
        assert!(!kr.is_trust_all_active());
    }

    #[test]
    fn trust_all_per_peer() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.set_trust_all_for_peer("peer1", true);
        assert!(kr.is_trust_all_for_peer("peer1"));
        assert!(!kr.is_trust_all_for_peer("peer2"));
        kr.set_trust_all_for_peer("peer1", false);
        assert!(!kr.is_trust_all_for_peer("peer1"));
    }

    #[test]
    fn resolve_found_active() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.supply_key("pki1", "user1", &valid_hex(), KeyTrustLevel::Explicit, None).unwrap();
        match kr.resolve_for_verification("pki1", "user1").unwrap() {
            KeyResolutionResult::Found(e) => assert_eq!(e.pki_id, "pki1"),
            _ => panic!("expected Found"),
        }
    }

    #[test]
    fn resolve_not_found() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        match kr.resolve_for_verification("missing", "user").unwrap() {
            KeyResolutionResult::NotFound => {}
            _ => panic!("expected NotFound"),
        }
    }

    #[test]
    fn resolve_expired() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.supply_key(
            "pki1",
            "user1",
            &valid_hex(),
            KeyTrustLevel::Explicit,
            Some("2020-01-01T00:00:00Z".to_string()),
        ).unwrap();
        match kr.resolve_for_verification("pki1", "user1").unwrap() {
            KeyResolutionResult::Expired => {}
            _ => panic!("expected Expired"),
        }
    }

    #[test]
    fn resolve_future_expiry_is_found() {
        let (engine, id_gen, _dir) = setup();
        let kr = KeyResolverEngine::new(engine, id_gen).unwrap();
        kr.supply_key(
            "pki1",
            "user1",
            &valid_hex(),
            KeyTrustLevel::Explicit,
            Some("2099-01-01T00:00:00Z".to_string()),
        ).unwrap();
        match kr.resolve_for_verification("pki1", "user1").unwrap() {
            KeyResolutionResult::Found(_) => {}
            _ => panic!("expected Found"),
        }
    }
}
