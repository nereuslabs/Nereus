//! Nereus Storage: SQLite WAL and in-memory persistence backends.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

#![deny(warnings)]

pub mod memory;
pub mod models;
pub mod sqlite;

pub use memory::MemoryStorage;
pub use models::{AssessmentRecord, SessionRecord, SessionStore};
pub use sqlite::{Migration, SqliteStorage, CURRENT_SCHEMA_VERSION, MIGRATIONS};
