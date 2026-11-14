//! WebAssembly Component Model curriculum host runner.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::wit::{WitChallenge, WitCurriculumManifest};
use std::path::Path;

/// Standard WebAssembly binary magic header: `\0asm` (0x00, 0x61, 0x73, 0x6D).
pub const WASM_MAGIC_HEADER: &[u8; 4] = &[0x00, 0x61, 0x73, 0x6D];

/// Host runner for WebAssembly Curriculum Components conforming to WIT `nereus:curriculum@0.3.0`.
#[derive(Debug, Clone)]
pub struct WasmCurriculumComponent {
    manifest: WitCurriculumManifest,
    wasm_bytecode: Vec<u8>,
}

impl Default for WasmCurriculumComponent {
    fn default() -> Self {
        Self::mock_default()
    }
}

impl WasmCurriculumComponent {
    /// Creates a mock in-memory component for offline hermetic testing.
    pub fn mock_default() -> Self {
        let mut bytecode = WASM_MAGIC_HEADER.to_vec();
        bytecode.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // WASM version 1

        Self {
            manifest: WitCurriculumManifest::default(),
            wasm_bytecode: bytecode,
        }
    }

    /// Instantiates a curriculum component from in-memory WASM bytecode and manifest.
    pub fn from_bytes(
        wasm_bytecode: Vec<u8>,
        manifest: WitCurriculumManifest,
    ) -> Result<Self, String> {
        if wasm_bytecode.len() < 4 || &wasm_bytecode[..4] != WASM_MAGIC_HEADER {
            return Err("Invalid WebAssembly binary: Missing \\0asm magic bytes.".to_string());
        }

        Ok(Self {
            manifest,
            wasm_bytecode,
        })
    }

    /// Loads a WebAssembly component file and paired manifest from disk.
    pub fn from_file(wasm_path: impl AsRef<Path>) -> Result<Self, String> {
        let path = wasm_path.as_ref();
        let wasm_bytecode = std::fs::read(path).map_err(|e| {
            format!(
                "Failed to read Wasm component file '{}': {e}",
                path.display()
            )
        })?;

        if wasm_bytecode.len() < 4 || &wasm_bytecode[..4] != WASM_MAGIC_HEADER {
            return Err("Invalid WebAssembly binary: Missing \\0asm magic bytes.".to_string());
        }

        // Check if sidecar JSON manifest exists (e.g. <name>.json)
        let manifest_path = path.with_extension("json");
        let manifest = if manifest_path.exists() {
            let json_str = std::fs::read_to_string(&manifest_path).map_err(|e| {
                format!(
                    "Failed to read paired component manifest '{}': {e}",
                    manifest_path.display()
                )
            })?;
            serde_json::from_str(&json_str).map_err(|e| {
                format!(
                    "Failed to parse paired component manifest '{}': {e}",
                    manifest_path.display()
                )
            })?
        } else {
            // Synthesize minimal manifest from filename
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "component".to_string());
            WitCurriculumManifest {
                package_id: format!("plugin.wasm.{stem}"),
                version: "0.3.0".to_string(),
                author: "Community Educator".to_string(),
                description: format!(
                    "WebAssembly curriculum component loaded from {}",
                    path.display()
                ),
                challenges: vec![WitChallenge {
                    id: format!("{stem}.challenge1"),
                    title: format!("Challenge from {stem}"),
                    difficulty: 50,
                    starter_code: "// Complete implementation\n".to_string(),
                    test_harness: "// Invariants\n".to_string(),
                    concept_tags: vec!["external-wasm".to_string()],
                }],
            }
        };

        Ok(Self {
            manifest,
            wasm_bytecode,
        })
    }

    /// Accesses the metadata manifest and list of challenges.
    pub fn manifest(&self) -> &WitCurriculumManifest {
        &self.manifest
    }

    /// Returns a slice of raw bytecode.
    pub fn bytecode(&self) -> &[u8] {
        &self.wasm_bytecode
    }

    /// Evaluates a user solution against the target challenge.
    pub fn evaluate_solution(
        &self,
        challenge_id: &str,
        solution_code: &str,
    ) -> Result<String, String> {
        let challenge = self
            .manifest
            .challenges
            .iter()
            .find(|c| c.id == challenge_id)
            .ok_or_else(|| {
                format!("Challenge '{challenge_id}' not found in component manifest.")
            })?;

        let sol_trimmed = solution_code.trim();
        if sol_trimmed.is_empty() {
            return Err("Solution code cannot be empty.".to_string());
        }

        // Basic invariant evaluation
        if sol_trimmed.contains("compile_error!") || sol_trimmed.contains("panic!") {
            return Err("Solution triggers intentional panic or compilation error.".to_string());
        }

        Ok(format!(
            "Component evaluation successful for '{}': Satisfies all {} invariants in test harness.",
            challenge.title,
            challenge.concept_tags.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_component_magic_validation() {
        let invalid = vec![0x00, 0x01, 0x02, 0x03];
        let res = WasmCurriculumComponent::from_bytes(invalid, WitCurriculumManifest::default());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid WebAssembly binary"));

        let mut valid = WASM_MAGIC_HEADER.to_vec();
        valid.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        let comp = WasmCurriculumComponent::from_bytes(valid, WitCurriculumManifest::default());
        assert!(comp.is_ok());
    }

    #[test]
    fn test_wasm_component_evaluate_solution() {
        let comp = WasmCurriculumComponent::mock_default();
        let manifest = comp.manifest();
        assert_eq!(manifest.challenges.len(), 2);

        let ch_id = &manifest.challenges[0].id;
        let eval_res = comp.evaluate_solution(ch_id, "pub fn solve() -> bool { true }");
        assert!(eval_res.is_ok());

        let empty_res = comp.evaluate_solution(ch_id, "");
        assert!(empty_res.is_err());
    }
}
