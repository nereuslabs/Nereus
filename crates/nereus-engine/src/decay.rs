//! Spaced Repetition and Ebbinghaus Concept Decay Engine for Nereus 2.0.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
//!
//! Models memory retention decay over time according to the exponential forgetting curve:
//! `R(t) = 2^(-t / S)`
//! where:
//! - `t` is elapsed days since last review
//! - `S` is stability (half-life in days)
//!
//! When `R(t) < threshold` (default 0.60), concepts are marked as decayed, prompting
//! `CoachAgent` to schedule micro-drills.

use chrono::{DateTime, Utc};
use nereus_core::models::{ConceptMastery, RoadmapTopic};

/// Default retention threshold below which a concept is considered decayed (60%).
pub const DEFAULT_DECAY_THRESHOLD: f64 = 0.60;

/// Default minimum stability in days.
pub const MINIMUM_STABILITY_DAYS: f64 = 1.0;

/// Engine calculating memory retention decay and updating concept stability.
#[derive(Debug, Clone)]
pub struct EbbinghausDecayEngine {
    decay_threshold: f64,
}

impl Default for EbbinghausDecayEngine {
    fn default() -> Self {
        Self {
            decay_threshold: DEFAULT_DECAY_THRESHOLD,
        }
    }
}

impl EbbinghausDecayEngine {
    /// Creates a new decay engine with custom retention threshold.
    pub fn new(threshold: f64) -> Self {
        Self {
            decay_threshold: threshold.clamp(0.10, 0.95),
        }
    }

    /// Computes memory retention probability `R(t) = 2^(-t / S)`.
    pub fn calculate_retention(elapsed_days: f64, stability_days: f64) -> f64 {
        if elapsed_days <= 0.0 {
            return 1.0;
        }
        let stability = stability_days.max(0.1);
        let retention = 2.0_f64.powf(-elapsed_days / stability);
        retention.clamp(0.0, 1.0)
    }

    /// Updates stability factor `S'` after a review based on score.
    pub fn update_stability(previous_stability: f64, score: f64) -> f64 {
        let prev = previous_stability.max(MINIMUM_STABILITY_DAYS);
        if score >= 70.0 {
            // Successful recall increases stability proportionally to score
            let multiplier = 1.0 + 1.2 * (score / 100.0);
            prev * multiplier
        } else {
            // Failed recall penalties stability down toward baseline
            (prev * 0.5).max(MINIMUM_STABILITY_DAYS)
        }
    }

    /// Calculates elapsed days between two ISO 8601 UTC timestamps.
    pub fn elapsed_days(start_iso: &str, end_iso: &str) -> f64 {
        let start = match DateTime::parse_from_rfc3339(start_iso) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => return 0.0,
        };
        let end = match DateTime::parse_from_rfc3339(end_iso) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => return 0.0,
        };
        let duration = end.signed_duration_since(start);
        (duration.num_seconds() as f64) / 86400.0
    }

    /// Evaluates whether a given concept mastery has decayed below retention threshold at `current_time`.
    pub fn is_decayed(&self, mastery: &ConceptMastery, current_time_iso: &str) -> bool {
        let elapsed = Self::elapsed_days(&mastery.last_reviewed_at, current_time_iso);
        let retention = Self::calculate_retention(elapsed, mastery.stability_days);
        retention < self.decay_threshold
    }

    /// Filters a list of concept masteries to return those that have decayed.
    pub fn filter_decayed<'a>(
        &self,
        masteries: &'a [ConceptMastery],
        current_time_iso: &str,
    ) -> Vec<&'a ConceptMastery> {
        masteries
            .iter()
            .filter(|m| self.is_decayed(m, current_time_iso))
            .collect()
    }

    /// Generates a bite-sized 60-second micro-drill topic targeting a decayed concept.
    pub fn generate_micro_drill(&self, mastery: &ConceptMastery) -> RoadmapTopic {
        let formatted_title = mastery
            .concept_id
            .split('.')
            .next_back()
            .unwrap_or(&mastery.concept_id)
            .replace('_', " ");

        RoadmapTopic {
            id: format!("micro-drill-{}", mastery.concept_id.replace('.', "-")),
            title: format!("⚡ Micro-Drill: {} (Spaced Review)", formatted_title),
            description: format!(
                "Reinforce retention for concept '{}'. Ebbinghaus model indicates memory decay. Complete rapid diagnostic check to restore stability.",
                mastery.concept_id
            ),
            difficulty: 0.3,
            prerequisites: vec![],
            estimated_hours: 0.1, // ~6 minutes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_formula_at_t_zero() {
        let r = EbbinghausDecayEngine::calculate_retention(0.0, 3.0);
        assert_eq!(r, 1.0);
    }

    #[test]
    fn test_retention_formula_at_half_life() {
        // At elapsed_days == stability_days, retention should be exactly 0.5 (50%)
        let r = EbbinghausDecayEngine::calculate_retention(4.0, 4.0);
        assert!((r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_decay_detection() {
        let engine = EbbinghausDecayEngine::new(0.60);

        let fresh_mastery = ConceptMastery {
            concept_id: "rust.borrow.move".to_string(),
            score: 90.0,
            last_reviewed_at: "2026-03-01T00:00:00Z".to_string(),
            stability_days: 5.0,
            review_count: 1,
        };

        // 1 day later: retention = 2^(-1/5) = 0.87 > 0.60 (Not decayed)
        assert!(!engine.is_decayed(&fresh_mastery, "2026-03-02T00:00:00Z"));

        // 5 days later: retention = 0.50 < 0.60 (Decayed)
        assert!(engine.is_decayed(&fresh_mastery, "2026-03-06T00:00:00Z"));
    }

    #[test]
    fn test_stability_growth_on_high_score() {
        let prev_s = 2.0;
        let next_s = EbbinghausDecayEngine::update_stability(prev_s, 90.0);
        assert!(next_s > prev_s);
        // multiplier = 1.0 + 1.2 * 0.9 = 2.08 -> 2.0 * 2.08 = 4.16
        assert!((next_s - 4.16).abs() < 1e-4);
    }

    #[test]
    fn test_stability_penalty_on_failure() {
        let prev_s = 4.0;
        let next_s = EbbinghausDecayEngine::update_stability(prev_s, 40.0);
        assert_eq!(next_s, 2.0); // 4.0 * 0.5 = 2.0
    }

    #[test]
    fn test_micro_drill_generation() {
        let engine = EbbinghausDecayEngine::default();
        let mastery = ConceptMastery {
            concept_id: "rust.lifetime.elision".to_string(),
            score: 80.0,
            last_reviewed_at: "2026-03-01T00:00:00Z".to_string(),
            stability_days: 1.0,
            review_count: 1,
        };

        let drill = engine.generate_micro_drill(&mastery);
        assert!(drill.title.contains("Micro-Drill"));
        assert!(drill.id.contains("rust-lifetime-elision"));
        assert!(drill.estimated_hours <= 0.2);
    }
}
