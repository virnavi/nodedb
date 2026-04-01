use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePublicKeyEntry {
    pub id: i64,
    pub pki_id: String,
    pub user_id: String,
    pub public_key_hex: String,
    pub trust_level: KeyTrustLevel,
    pub cached_at_utc: String,
    pub expires_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyTrustLevel {
    Explicit,
    TrustAll,
    Revoked,
}

pub enum KeyResolutionResult {
    Found(NodePublicKeyEntry),
    NotFound,
    Expired,
}

impl KeyTrustLevel {
    pub fn from_str(s: &str) -> Self {
        match s {
            "explicit" => Self::Explicit,
            "trust_all" => Self::TrustAll,
            "revoked" => Self::Revoked,
            _ => Self::Explicit,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Explicit => "explicit",
            Self::TrustAll => "trust_all",
            Self::Revoked => "revoked",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_roundtrip() {
        let levels = vec![
            KeyTrustLevel::Explicit,
            KeyTrustLevel::TrustAll,
            KeyTrustLevel::Revoked,
        ];
        for l in levels {
            assert_eq!(KeyTrustLevel::from_str(l.as_str()), l);
        }
    }

    #[test]
    fn trust_level_unknown_defaults_to_explicit() {
        assert_eq!(KeyTrustLevel::from_str("garbage"), KeyTrustLevel::Explicit);
    }

    #[test]
    fn entry_serde_roundtrip() {
        let entry = NodePublicKeyEntry {
            id: 1,
            pki_id: "abc123".to_string(),
            user_id: "user-1".to_string(),
            public_key_hex: "a".repeat(64),
            trust_level: KeyTrustLevel::Explicit,
            cached_at_utc: "2025-01-01T00:00:00Z".to_string(),
            expires_at_utc: Some("2026-01-01T00:00:00Z".to_string()),
        };
        let bytes = rmp_serde::to_vec(&entry).unwrap();
        let decoded: NodePublicKeyEntry = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, 1);
        assert_eq!(decoded.pki_id, "abc123");
        assert_eq!(decoded.user_id, "user-1");
        assert_eq!(decoded.public_key_hex, "a".repeat(64));
        assert_eq!(decoded.trust_level, KeyTrustLevel::Explicit);
        assert_eq!(decoded.expires_at_utc, Some("2026-01-01T00:00:00Z".to_string()));
    }
}
