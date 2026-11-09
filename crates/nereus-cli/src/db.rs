//! Database path resolution and environment variable defaults.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use std::path::PathBuf;

/// Resolves the SQLite database path following the precedence:
/// 1. Explicit CLI argument (`--db <path>`)
/// 2. Environment variable (`NEREUS_DB_PATH`)
/// 3. Default user home location (`~/.nereus/nereus.db` or `.nereus/nereus.db`)
pub fn resolve_db_path(custom_path: Option<&str>) -> PathBuf {
    if let Some(path) = custom_path {
        expand_tilde(path)
    } else if let Ok(env_path) = std::env::var("NEREUS_DB_PATH") {
        expand_tilde(&env_path)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".nereus").join("nereus.db")
    } else {
        PathBuf::from(".nereus").join("nereus.db")
    }
}

/// Expands a leading tilde `~` or `~/` to the user's HOME directory if present.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_path_resolution() {
        let p = resolve_db_path(Some("/custom/nereus.db"));
        assert_eq!(p, PathBuf::from("/custom/nereus.db"));
    }

    #[test]
    fn test_expand_tilde() {
        if let Ok(home) = std::env::var("HOME") {
            let expanded = expand_tilde("~/my_db.sqlite");
            assert_eq!(expanded, PathBuf::from(home).join("my_db.sqlite"));

            let just_tilde = expand_tilde("~");
            assert_eq!(just_tilde, PathBuf::from(std::env::var("HOME").unwrap()));
        }

        let normal = expand_tilde("/absolute/path.db");
        assert_eq!(normal, PathBuf::from("/absolute/path.db"));
    }
}
