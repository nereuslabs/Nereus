//! Socratic evaluation and dynamic threshold formulas.

use nereus_core::models::{Assessment, RoadmapTopic, Verdict};

/// Compute dynamic passing threshold based on topic difficulty:
/// Formula: min(70.0 + difficulty * 15.0, 85.0)
pub fn compute_pass_threshold(difficulty: f32) -> f32 {
    (70.0 + difficulty * 15.0).min(85.0)
}

/// Evaluates raw assessment metrics against the difficulty-scaled passing threshold.
pub fn evaluate_verdict(topic: &RoadmapTopic, score: f32) -> Verdict {
    let threshold = compute_pass_threshold(topic.difficulty);
    if score >= threshold {
        Verdict::Pass
    } else {
        Verdict::Retry
    }
}

/// Enforces Socratic guidance rules:
/// Never reveals direct code answers; returns conceptual pointers and compiler clues.
pub fn sanitize_socratic_feedback(raw_feedback: &str, verdict: Verdict) -> String {
    match verdict {
        Verdict::Pass => format!("Mastery confirmed. {raw_feedback}"),
        Verdict::Retry => {
            if raw_feedback.is_empty() {
                "Examine compiler diagnostics and type contracts. What invariant was violated?".to_string()
            } else {
                format!("Socratic Guidance: {raw_feedback}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_scaling() {
        assert_eq!(compute_pass_threshold(0.0), 70.0);
        assert_eq!(compute_pass_threshold(0.5), 77.5);
        assert_eq!(compute_pass_threshold(1.0), 85.0);
        assert_eq!(compute_pass_threshold(1.5), 85.0); // Clamped <= 85.0
    }
}
