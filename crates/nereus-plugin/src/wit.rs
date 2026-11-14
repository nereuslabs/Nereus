//! WebAssembly Interface Types (WIT) definitions and manifest schemas.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use nereus_core::models::RoadmapTopic;
use serde::{Deserialize, Serialize};

/// Canonical WIT document for Nereus 2.0 WebAssembly Curriculum Components.
pub const CANONICAL_WIT_SPECIFICATION: &str = r#"
package nereus:curriculum@0.3.0;

interface topic-provider {
    record challenge {
        id: string,
        title: string,
        difficulty: u32,
        starter-code: string,
        test-harness: string,
        concept-tags: list<string>,
    }

    get-manifest: func() -> list<challenge>;
    evaluate-solution: func(code: string) -> result<string, string>;
}

world curriculum-host {
    export topic-provider;
}
"#;

/// A challenge definition conforming to the WIT `challenge` record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitChallenge {
    /// Unique topic / exercise identifier (e.g. "rust.advanced.lockfree").
    pub id: String,
    /// Human-readable title of the challenge.
    pub title: String,
    /// Difficulty rating from 1 to 100.
    pub difficulty: u32,
    /// Starter code scaffold presented in the learner editor.
    pub starter_code: String,
    /// Test harness and invariants used to verify solutions.
    pub test_harness: String,
    /// Associated concept tags (e.g. ["concurrency", "atomics"]).
    pub concept_tags: Vec<String>,
}

impl WitChallenge {
    /// Converts this WIT challenge record into a core `RoadmapTopic`.
    pub fn to_roadmap_topic(&self) -> RoadmapTopic {
        let diff_float = (self.difficulty as f32 / 100.0).clamp(0.1, 1.0);
        RoadmapTopic {
            id: self.id.clone(),
            title: self.title.clone(),
            description: format!("Component Model Challenge: {}", self.title),
            difficulty: diff_float,
            prerequisites: vec![],
            estimated_hours: (self.difficulty as f32 / 20.0).max(1.0),
        }
    }
}

/// Metadata manifest describing a curriculum plugin component package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitCurriculumManifest {
    /// Plugin package identifier (e.g. "org.mit.systems-curriculum").
    pub package_id: String,
    /// Semantic version (e.g. "0.3.0").
    pub version: String,
    /// Author or sponsoring organization.
    pub author: String,
    /// Description of the course track.
    pub description: String,
    /// List of exported challenges.
    pub challenges: Vec<WitChallenge>,
}

impl Default for WitCurriculumManifest {
    fn default() -> Self {
        Self {
            package_id: "nereus:curriculum".to_string(),
            version: "0.3.0".to_string(),
            author: "Nereus Labs".to_string(),
            description: "Default WebAssembly Curriculum Component".to_string(),
            challenges: vec![
                WitChallenge {
                    id: "component.atomics.cas".to_string(),
                    title: "Lock-Free Compare-And-Swap Stack".to_string(),
                    difficulty: 85,
                    starter_code:
                        "pub struct CasStack;\nimpl CasStack { pub fn new() -> Self { Self } }"
                            .to_string(),
                    test_harness: "#[test] fn test_cas() { assert!(true); }".to_string(),
                    concept_tags: vec!["atomics".to_string(), "lock-free".to_string()],
                },
                WitChallenge {
                    id: "component.async.waker".to_string(),
                    title: "Manual Cooperative Task Waker".to_string(),
                    difficulty: 90,
                    starter_code:
                        "pub struct CustomWaker;\nimpl CustomWaker { pub fn wake(&self) {} }"
                            .to_string(),
                    test_harness: "#[test] fn test_waker() { assert!(true); }".to_string(),
                    concept_tags: vec!["async".to_string(), "waker".to_string()],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wit_specification_contains_package_and_interface() {
        assert!(CANONICAL_WIT_SPECIFICATION.contains("package nereus:curriculum@0.3.0;"));
        assert!(CANONICAL_WIT_SPECIFICATION.contains("interface topic-provider"));
        assert!(CANONICAL_WIT_SPECIFICATION.contains("get-manifest"));
        assert!(CANONICAL_WIT_SPECIFICATION.contains("evaluate-solution"));
    }

    #[test]
    fn test_wit_challenge_to_roadmap_topic() {
        let challenge = WitChallenge {
            id: "wasm.threads.atomic".to_string(),
            title: "Atomics in Wasm".to_string(),
            difficulty: 80,
            starter_code: "".to_string(),
            test_harness: "".to_string(),
            concept_tags: vec!["wasm".to_string()],
        };

        let topic = challenge.to_roadmap_topic();
        assert_eq!(topic.id, "wasm.threads.atomic");
        assert_eq!(topic.title, "Atomics in Wasm");
        assert_eq!(topic.difficulty, 0.8);
        assert_eq!(topic.estimated_hours, 4.0);
    }
}
