use crate::types::ProvenanceSourceType;

/// Compute initial confidence based on source type, signature status, and hop count.
pub fn initial_confidence(source_type: &ProvenanceSourceType, is_signed: bool, hops: u8) -> f64 {
    let base = match (source_type, is_signed) {
        (ProvenanceSourceType::User, true) => 1.0,
        (ProvenanceSourceType::User, false) => 0.85,
        (ProvenanceSourceType::Peer, true) => 0.90,
        (ProvenanceSourceType::Peer, false) => 0.60,
        (ProvenanceSourceType::Model, true) => 0.75,
        (ProvenanceSourceType::Model, false) => 0.50,
        (ProvenanceSourceType::Import, _) => 0.70,
        (ProvenanceSourceType::Sensor, true) => 0.90,
        (ProvenanceSourceType::Sensor, false) => 0.60,
        (ProvenanceSourceType::AiQuery, _) => 0.50,
        (ProvenanceSourceType::Unknown, _) => 0.40,
    };

    // Apply per-hop decay for federated sources (hops > 0)
    if hops > 0 {
        // First hop already accounted for in base; additional hops get 0.85 decay each
        let additional_hops = (hops - 1) as f64;
        base * 0.85_f64.powf(additional_hops)
    } else {
        base
    }
}

/// Corroborate: increase confidence using Noisy-OR formula.
/// Returns: 1.0 - (1.0 - current) * (1.0 - new_source_confidence)
pub fn corroborate(current: f64, new_source_confidence: f64) -> f64 {
    let result = 1.0 - (1.0 - current) * (1.0 - new_source_confidence);
    result.clamp(0.0, 1.0)
}

/// Conflict: decrease confidence proportional to discrepancy magnitude.
/// Returns: current * (1.0 - magnitude), clamped to [0.0, 1.0]
pub fn conflict(current: f64, magnitude: f64) -> f64 {
    let result = current * (1.0 - magnitude);
    result.clamp(0.0, 1.0)
}

/// Boost confidence after successful signature verification.
/// Returns: min(current + 0.10, 1.0)
pub fn verification_boost(current: f64) -> f64 {
    (current + 0.10).min(1.0)
}

/// Set confidence to 0.0 on signature verification failure.
pub fn verification_failure() -> f64 {
    0.0
}

/// Apply age decay: current * 0.5^(days_elapsed / half_life_days)
pub fn age_decay(current: f64, days_elapsed: f64, half_life_days: f64) -> f64 {
    if half_life_days <= 0.0 || days_elapsed <= 0.0 {
        return current;
    }
    let result = current * (0.5_f64).powf(days_elapsed / half_life_days);
    result.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_user_signed() {
        assert!((initial_confidence(&ProvenanceSourceType::User, true, 0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_user_unsigned() {
        assert!((initial_confidence(&ProvenanceSourceType::User, false, 0) - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_peer_signed_one_hop() {
        assert!((initial_confidence(&ProvenanceSourceType::Peer, true, 1) - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_peer_signed_two_hops() {
        let expected = 0.90 * 0.85;
        assert!((initial_confidence(&ProvenanceSourceType::Peer, true, 2) - expected).abs() < 1e-10);
    }

    #[test]
    fn initial_peer_unsigned() {
        assert!((initial_confidence(&ProvenanceSourceType::Peer, false, 1) - 0.60).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_model_signed() {
        assert!((initial_confidence(&ProvenanceSourceType::Model, true, 0) - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_model_unsigned() {
        assert!((initial_confidence(&ProvenanceSourceType::Model, false, 0) - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_import() {
        assert!((initial_confidence(&ProvenanceSourceType::Import, false, 0) - 0.70).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_ai_query() {
        assert!((initial_confidence(&ProvenanceSourceType::AiQuery, false, 0) - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn initial_unknown() {
        assert!((initial_confidence(&ProvenanceSourceType::Unknown, false, 0) - 0.40).abs() < f64::EPSILON);
    }

    #[test]
    fn corroborate_increases() {
        let result = corroborate(0.60, 0.70);
        // 1 - (1-0.6)*(1-0.7) = 1 - 0.4*0.3 = 1 - 0.12 = 0.88
        assert!((result - 0.88).abs() < 1e-10);
    }

    #[test]
    fn corroborate_at_one() {
        assert!((corroborate(1.0, 0.5) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn corroborate_at_zero() {
        assert!((corroborate(0.0, 0.7) - 0.7).abs() < 1e-10);
    }

    #[test]
    fn conflict_reduces() {
        let result = conflict(0.80, 0.5);
        // 0.80 * (1 - 0.5) = 0.40
        assert!((result - 0.40).abs() < 1e-10);
    }

    #[test]
    fn conflict_full_magnitude() {
        assert!((conflict(0.80, 1.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn conflict_zero_magnitude() {
        assert!((conflict(0.80, 0.0) - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn verification_boost_normal() {
        assert!((verification_boost(0.80) - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn verification_boost_capped() {
        assert!((verification_boost(0.95) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verification_failure_returns_zero() {
        assert!((verification_failure() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn age_decay_one_half_life() {
        let result = age_decay(1.0, 30.0, 30.0);
        assert!((result - 0.5).abs() < 1e-10);
    }

    #[test]
    fn age_decay_two_half_lives() {
        let result = age_decay(1.0, 60.0, 30.0);
        assert!((result - 0.25).abs() < 1e-10);
    }

    #[test]
    fn age_decay_zero_days() {
        assert!((age_decay(0.80, 0.0, 30.0) - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn age_decay_zero_half_life() {
        assert!((age_decay(0.80, 10.0, 0.0) - 0.80).abs() < f64::EPSILON);
    }
}
