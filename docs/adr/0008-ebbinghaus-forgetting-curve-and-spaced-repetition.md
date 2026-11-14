# ADR 0008: Ebbinghaus Forgetting Curve and Spaced Repetition Knowledge Graph

* **Status**: Accepted
* **Deciders**: Nereus Engineering Core, Project Maintainer
* **Date**: 2026-11-14

---

## Context & Problem Statement

Linear curriculum engines treat completed topics as permanently learned. In practice, technical knowledge—such as Rust ownership semantics, lifetime bounds, and trait coherence—decays exponentially without active recall (the Ebbinghaus Forgetting Curve). Learners progress through subsequent lessons only to encounter comprehension roadblocks rooted in forgotten prerequisite concepts.

## Decision Drivers

* Mathematical modeling of memory decay based on proven cognitive science.
* Minimal SQLite storage overhead and sub-millisecond query evaluation.
* Automatic injection of targeted micro-drills into the Coach's learning roadmap when concept retention drops below passing thresholds.

## Considered Options

1. **Static Review Intervals (Leitner 5-box system)**: Simple integer intervals, but too rigid for dynamic programming concepts where mastery depth varies wildly.
2. **SuperMemo SM-2 Algorithm**: Widely used in flashcard apps, but overly complex for continuous technical coding sessions and assumes binary flashcard grading.
3. **Continuous Ebbinghaus Exponential Retention Formula**:
   $$R(t) = 2^{-t / S}$$
   where $t$ is elapsed time since last review and $S$ is memory stability scaled by mastery score $M \in [0.0, 1.0]$.

## Decision Outcome

Chosen option: **Continuous Ebbinghaus Exponential Retention Formula with SQLite `concept_mastery` persistence**.

Implemented in `nereus-core` (`ConceptMastery` model), `nereus-storage` (schema migration v3), and `nereus-agents` (`CoachAgent`):

```rust
pub struct ConceptMastery {
    pub concept_id: String,
    pub mastery_score: f32, // 0.0 to 1.0
    pub last_reviewed_at: i64,
    pub stability_days: f32,
    pub review_count: u32,
}

impl ConceptMastery {
    pub fn retention_rate(&self, now_timestamp: i64) -> f32 {
        let elapsed_days = ((now_timestamp - self.last_reviewed_at) as f32) / 86400.0;
        let stability = self.stability_days.max(0.1);
        (0.5f32).powf(elapsed_days / stability)
    }
}
```

When generating roadmaps, `CoachAgent::generate_roadmap_with_decay` queries `SessionStore::get_decayed_concepts(threshold = 0.60)`. For each decayed concept, a prerequisite micro-drill is automatically prepended to the learning agenda.

### Consequences

* **Positive**: Learners automatically review deteriorating concepts before taking on advanced topics.
* **Positive**: Fully backward-compatible schema via SQLite migration `003_spaced_repetition.sql`.
* **Positive**: Hermetically verifiable with deterministic timestamps in unit tests.
* **Negative**: Requires persisting explicit assessment results per concept to update stability metrics accurately.
