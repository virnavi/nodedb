use serde::{Deserialize, Serialize};

/// Strategy for resolving conflicts between local and remote data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Most recent write wins (based on updated_at timestamp).
    LastWriteWins,
    /// Local value always wins.
    LocalWins,
    /// Remote value always wins.
    RemoteWins,
    /// Higher confidence value wins, falls back to LastWriteWins on tie.
    HighestConfidence,
    /// Requires manual resolution.
    Manual,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        ConflictResolution::LastWriteWins
    }
}

impl ConflictResolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictResolution::LastWriteWins => "last_write_wins",
            ConflictResolution::LocalWins => "local_wins",
            ConflictResolution::RemoteWins => "remote_wins",
            ConflictResolution::HighestConfidence => "highest_confidence",
            ConflictResolution::Manual => "manual",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "last_write_wins" => Some(ConflictResolution::LastWriteWins),
            "local_wins" => Some(ConflictResolution::LocalWins),
            "remote_wins" => Some(ConflictResolution::RemoteWins),
            "highest_confidence" => Some(ConflictResolution::HighestConfidence),
            "manual" => Some(ConflictResolution::Manual),
            _ => None,
        }
    }
}

/// Context for conflict resolution decisions.
#[derive(Debug, Clone)]
pub struct ConflictContext {
    /// When the local value was last updated (milliseconds since epoch).
    pub local_updated_at: i64,
    /// When the remote value was last updated (milliseconds since epoch).
    pub remote_updated_at: i64,
    /// Confidence score of the local value (0.0..=1.0), if available.
    pub local_confidence: Option<f64>,
    /// Confidence score of the remote value (0.0..=1.0), if available.
    pub remote_confidence: Option<f64>,
}

/// Outcome of conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictOutcome {
    /// Keep the local value.
    KeepLocal,
    /// Accept the remote value.
    AcceptRemote,
    /// Cannot be resolved automatically; requires manual intervention.
    RequiresManual,
}

/// Resolve a conflict between local and remote data.
pub fn resolve_conflict(strategy: ConflictResolution, ctx: &ConflictContext) -> ConflictOutcome {
    match strategy {
        ConflictResolution::LastWriteWins => {
            if ctx.remote_updated_at > ctx.local_updated_at {
                ConflictOutcome::AcceptRemote
            } else {
                ConflictOutcome::KeepLocal
            }
        }
        ConflictResolution::LocalWins => ConflictOutcome::KeepLocal,
        ConflictResolution::RemoteWins => ConflictOutcome::AcceptRemote,
        ConflictResolution::HighestConfidence => {
            match (ctx.local_confidence, ctx.remote_confidence) {
                (Some(l), Some(r)) => {
                    if r > l {
                        ConflictOutcome::AcceptRemote
                    } else if l > r {
                        ConflictOutcome::KeepLocal
                    } else {
                        // Tie: fall back to LWW
                        if ctx.remote_updated_at > ctx.local_updated_at {
                            ConflictOutcome::AcceptRemote
                        } else {
                            ConflictOutcome::KeepLocal
                        }
                    }
                }
                // If confidence is missing, fall back to LWW
                _ => {
                    if ctx.remote_updated_at > ctx.local_updated_at {
                        ConflictOutcome::AcceptRemote
                    } else {
                        ConflictOutcome::KeepLocal
                    }
                }
            }
        }
        ConflictResolution::Manual => ConflictOutcome::RequiresManual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(local_ts: i64, remote_ts: i64) -> ConflictContext {
        ConflictContext {
            local_updated_at: local_ts,
            remote_updated_at: remote_ts,
            local_confidence: None,
            remote_confidence: None,
        }
    }

    fn ctx_with_confidence(
        local_ts: i64,
        remote_ts: i64,
        local_conf: Option<f64>,
        remote_conf: Option<f64>,
    ) -> ConflictContext {
        ConflictContext {
            local_updated_at: local_ts,
            remote_updated_at: remote_ts,
            local_confidence: local_conf,
            remote_confidence: remote_conf,
        }
    }

    #[test]
    fn lww_newer_remote_wins() {
        let outcome = resolve_conflict(ConflictResolution::LastWriteWins, &ctx(100, 200));
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn lww_newer_local_wins() {
        let outcome = resolve_conflict(ConflictResolution::LastWriteWins, &ctx(200, 100));
        assert_eq!(outcome, ConflictOutcome::KeepLocal);
    }

    #[test]
    fn lww_equal_timestamp_keeps_local() {
        let outcome = resolve_conflict(ConflictResolution::LastWriteWins, &ctx(100, 100));
        assert_eq!(outcome, ConflictOutcome::KeepLocal);
    }

    #[test]
    fn local_wins_always() {
        let outcome = resolve_conflict(ConflictResolution::LocalWins, &ctx(100, 200));
        assert_eq!(outcome, ConflictOutcome::KeepLocal);
    }

    #[test]
    fn remote_wins_always() {
        let outcome = resolve_conflict(ConflictResolution::RemoteWins, &ctx(200, 100));
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn highest_confidence_remote_higher() {
        let outcome = resolve_conflict(
            ConflictResolution::HighestConfidence,
            &ctx_with_confidence(100, 200, Some(0.5), Some(0.9)),
        );
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn highest_confidence_local_higher() {
        let outcome = resolve_conflict(
            ConflictResolution::HighestConfidence,
            &ctx_with_confidence(100, 200, Some(0.9), Some(0.5)),
        );
        assert_eq!(outcome, ConflictOutcome::KeepLocal);
    }

    #[test]
    fn highest_confidence_tie_falls_back_to_lww() {
        let outcome = resolve_conflict(
            ConflictResolution::HighestConfidence,
            &ctx_with_confidence(100, 200, Some(0.8), Some(0.8)),
        );
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn highest_confidence_none_falls_back_to_lww() {
        let outcome = resolve_conflict(
            ConflictResolution::HighestConfidence,
            &ctx(100, 200),
        );
        assert_eq!(outcome, ConflictOutcome::AcceptRemote);
    }

    #[test]
    fn manual_always_requires_manual() {
        let outcome = resolve_conflict(ConflictResolution::Manual, &ctx(100, 200));
        assert_eq!(outcome, ConflictOutcome::RequiresManual);
    }

    #[test]
    fn default_is_lww() {
        assert_eq!(ConflictResolution::default(), ConflictResolution::LastWriteWins);
    }

    #[test]
    fn serde_roundtrip() {
        let strategy = ConflictResolution::HighestConfidence;
        let bytes = rmp_serde::to_vec(&strategy).unwrap();
        let decoded: ConflictResolution = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded, strategy);
    }

    #[test]
    fn as_str_from_str_roundtrip() {
        let variants = [
            ConflictResolution::LastWriteWins,
            ConflictResolution::LocalWins,
            ConflictResolution::RemoteWins,
            ConflictResolution::HighestConfidence,
            ConflictResolution::Manual,
        ];
        for v in &variants {
            let s = v.as_str();
            let parsed = ConflictResolution::from_str(s).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn from_str_unknown_returns_none() {
        assert!(ConflictResolution::from_str("unknown").is_none());
    }
}
