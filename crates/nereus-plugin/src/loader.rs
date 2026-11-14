//! Dynamic plugin and WebAssembly curriculum extension loader.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use crate::component::WasmCurriculumComponent;
use nereus_core::models::RoadmapTopic;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Dynamic loader that scans plugin directories for custom WebAssembly curriculum packages.
#[derive(Debug, Clone)]
pub struct PluginExtensionLoader {
    plugin_dir: PathBuf,
}

impl Default for PluginExtensionLoader {
    fn default() -> Self {
        Self::new(Self::default_plugin_dir())
    }
}

impl PluginExtensionLoader {
    /// Constructs a loader targeting a specific plugin directory.
    pub fn new(plugin_dir: impl AsRef<Path>) -> Self {
        Self {
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }

    /// Resolves default plugin directory `~/.nereus/plugins`.
    pub fn default_plugin_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".nereus").join("plugins")
        } else {
            PathBuf::from(".nereus_plugins")
        }
    }

    /// Access the configured plugin directory.
    pub fn plugin_dir(&self) -> &Path {
        &self.plugin_dir
    }

    /// Scans the plugin directory and loads all valid `.wasm` curriculum components.
    pub fn discover_components(&self) -> Vec<WasmCurriculumComponent> {
        let mut components = Vec::new();

        if !self.plugin_dir.exists() {
            return components;
        }

        let entries = match std::fs::read_dir(&self.plugin_dir) {
            Ok(e) => e,
            Err(err) => {
                warn!(
                    "Failed to read plugin directory '{}': {err}",
                    self.plugin_dir.display()
                );
                return components;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "wasm" {
                        match WasmCurriculumComponent::from_file(&path) {
                            Ok(comp) => {
                                info!(
                                    "Discovered and loaded Wasm curriculum component: '{}' ({})",
                                    comp.manifest().package_id,
                                    path.display()
                                );
                                components.push(comp);
                            }
                            Err(err) => {
                                warn!("Skipping invalid component '{}': {err}", path.display());
                            }
                        }
                    }
                }
            }
        }

        components
    }

    /// Extracts all custom `RoadmapTopic` instances exported by discovered components.
    pub fn load_dynamic_topics(&self) -> Vec<RoadmapTopic> {
        let components = self.discover_components();
        let mut topics = Vec::new();

        for comp in components {
            for challenge in &comp.manifest().challenges {
                topics.push(challenge.to_roadmap_topic());
            }
        }

        topics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::WASM_MAGIC_HEADER;
    use crate::wit::WitCurriculumManifest;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_plugin_extension_loader_empty_dir() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!("nereus-plugin-test-{nanos}"));
        let loader = PluginExtensionLoader::new(&temp);
        assert!(loader.discover_components().is_empty());
        assert!(loader.load_dynamic_topics().is_empty());
    }

    #[test]
    fn test_plugin_extension_loader_discovers_wasm_component() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!("nereus-plugin-test-{nanos}"));
        std::fs::create_dir_all(&temp).unwrap();

        let wasm_file = temp.join("custom_course.wasm");
        let mut wasm_bytes = WASM_MAGIC_HEADER.to_vec();
        wasm_bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        std::fs::write(&wasm_file, wasm_bytes).unwrap();

        // Write paired manifest JSON
        let json_file = temp.join("custom_course.json");
        let manifest = WitCurriculumManifest::default();
        std::fs::write(&json_file, serde_json::to_string(&manifest).unwrap()).unwrap();

        let loader = PluginExtensionLoader::new(&temp);
        let components = loader.discover_components();
        assert_eq!(components.len(), 1);

        let topics = loader.load_dynamic_topics();
        assert_eq!(topics.len(), 2);
        assert_eq!(topics[0].id, "component.atomics.cas");

        let _ = std::fs::remove_dir_all(&temp);
    }
}
