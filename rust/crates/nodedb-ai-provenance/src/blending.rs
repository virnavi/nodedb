use nodedb_provenance::ProvenanceVerificationStatus;

/// Blend deterministic and AI-suggested confidence values.
///
/// Formula: `deterministic * (1 - weight) + ai_suggested * weight`, clamped [0.0, 1.0].
/// If `verification_failed` is true, always returns 0.0.
pub fn blend_confidence(
    deterministic: f64,
    ai_suggested: f64,
    weight: f64,
    verification_failed: bool,
) -> f64 {
    if verification_failed {
        return 0.0;
    }
    let blended = deterministic * (1.0 - weight) + ai_suggested * weight;
    blended.clamp(0.0, 1.0)
}

/// Apply the verification boost (+0.10) AFTER blending, clamped to 1.0.
pub fn apply_verification_boost_post_blend(
    blended: f64,
    status: &ProvenanceVerificationStatus,
) -> f64 {
    match status {
        ProvenanceVerificationStatus::Verified => (blended + 0.10).min(1.0),
        ProvenanceVerificationStatus::Failed => 0.0,
        _ => blended,
    }
}

/// Apply anomaly penalty to a confidence value, clamped to [0.0, 1.0].
pub fn apply_anomaly_penalty(confidence: f64, penalty: f64) -> f64 {
    (confidence - penalty).clamp(0.0, 1.0)
}

/// Apply conflict deltas to two confidence values, returning (new_a, new_b).
pub fn apply_conflict_deltas(
    confidence_a: f64,
    confidence_b: f64,
    delta_a: f64,
    delta_b: f64,
) -> (f64, f64) {
    let new_a = (confidence_a + delta_a).clamp(0.0, 1.0);
    let new_b = (confidence_b + delta_b).clamp(0.0, 1.0);
    (new_a, new_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_basic() {
        // deterministic=0.7, ai=0.9, weight=0.3
        // 0.7 * 0.7 + 0.9 * 0.3 = 0.49 + 0.27 = 0.76
        let result = blend_confidence(0.7, 0.9, 0.3, false);
        assert!((result - 0.76).abs() < 1e-10);
    }

    #[test]
    fn blend_zero_weight_is_deterministic() {
        let result = blend_confidence(0.7, 0.9, 0.0, false);
        assert!((result - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn blend_full_weight_is_ai() {
        let result = blend_confidence(0.7, 0.9, 1.0, false);
        assert!((result - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn blend_verification_failed_forces_zero() {
        let result = blend_confidence(0.7, 0.9, 0.3, true);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn blend_clamped_to_one() {
        let result = blend_confidence(1.0, 1.5, 0.5, false);
        assert!(result <= 1.0);
    }

    #[test]
    fn blend_clamped_to_zero() {
        let result = blend_confidence(-0.5, -0.5, 0.5, false);
        assert!(result >= 0.0);
    }

    #[test]
    fn verification_boost_verified() {
        let result = apply_verification_boost_post_blend(0.76, &ProvenanceVerificationStatus::Verified);
        assert!((result - 0.86).abs() < 1e-10);
    }

    #[test]
    fn verification_boost_capped() {
        let result = apply_verification_boost_post_blend(0.95, &ProvenanceVerificationStatus::Verified);
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verification_boost_failed_forces_zero() {
        let result = apply_verification_boost_post_blend(0.76, &ProvenanceVerificationStatus::Failed);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verification_boost_unverified_no_change() {
        let result = apply_verification_boost_post_blend(0.76, &ProvenanceVerificationStatus::Unverified);
        assert!((result - 0.76).abs() < f64::EPSILON);
    }

    #[test]
    fn anomaly_penalty_basic() {
        let result = apply_anomaly_penalty(0.8, 0.2);
        assert!((result - 0.6).abs() < 1e-10);
    }

    #[test]
    fn anomaly_penalty_clamped_to_zero() {
        let result = apply_anomaly_penalty(0.1, 0.5);
        assert!((result - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn conflict_deltas_basic() {
        let (a, b) = apply_conflict_deltas(0.8, 0.6, -0.1, 0.05);
        assert!((a - 0.7).abs() < 1e-10);
        assert!((b - 0.65).abs() < 1e-10);
    }

    #[test]
    fn conflict_deltas_clamped() {
        let (a, b) = apply_conflict_deltas(0.1, 0.9, -0.5, 0.5);
        assert!((a - 0.0).abs() < f64::EPSILON);
        assert!((b - 1.0).abs() < f64::EPSILON);
    }
}
