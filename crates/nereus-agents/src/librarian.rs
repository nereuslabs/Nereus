//! Offline Librarian Agent for Standard Library, Nomicon, and Reference citations.
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use serde::{Deserialize, Serialize};

/// Categorization of documentation entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocCategory {
    /// Concurrency primitives (`std::sync`).
    StdSync,
    /// Interior mutability types (`std::cell`).
    StdCell,
    /// Pinning and immovable types (`std::pin`).
    StdPin,
    /// Operator and lifecycle traits (`std::ops`).
    StdOps,
    /// Builtin marker traits and types (`std::marker`).
    StdMarker,
    /// Advanced and unsafe Rust patterns (The Rust Nomicon).
    Nomicon,
    /// Official language rules (The Rust Reference).
    RustReference,
    /// rustc diagnostic error code explanations.
    CompilerError,
}

impl std::fmt::Display for DocCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdSync => write!(f, "std::sync"),
            Self::StdCell => write!(f, "std::cell"),
            Self::StdPin => write!(f, "std::pin"),
            Self::StdOps => write!(f, "std::ops"),
            Self::StdMarker => write!(f, "std::marker"),
            Self::Nomicon => write!(f, "Nomicon"),
            Self::RustReference => write!(f, "Rust Reference"),
            Self::CompilerError => write!(f, "Compiler Diagnostic"),
        }
    }
}

/// A structured documentation reference entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocReference {
    /// Title or name of the concept / item.
    pub title: String,
    /// Canonical Rust path or diagnostic identifier (e.g., `std::sync::Arc`, `E0502`).
    pub path: String,
    /// Classification category.
    pub category: DocCategory,
    /// High-density technical summary.
    pub summary: String,
    /// Official documentation link.
    pub url: String,
    /// Optional concise idiomatic usage snippet.
    pub code_snippet: Option<String>,
    /// Search keywords for indexing.
    pub keywords: Vec<String>,
}

/// Socratic response from the Librarian Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibrarianResponse {
    /// The user or pedagogical prompt query.
    pub query: String,
    /// Primary documentation match, if found.
    pub primary_doc: Option<DocReference>,
    /// Secondary or related documentation items.
    pub related_docs: Vec<DocReference>,
    /// High-density Socratic synthesis guiding the learner to invariant principles.
    pub socratic_note: String,
}

/// Offline Librarian Agent providing instant, zero-cloud standard library search.
#[derive(Debug, Clone)]
pub struct LibrarianAgent {
    corpus: Vec<DocReference>,
}

impl Default for LibrarianAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl LibrarianAgent {
    /// Instantiates the Librarian Agent with the curated in-memory offline corpus.
    pub fn new() -> Self {
        Self {
            corpus: Self::build_canonical_corpus(),
        }
    }

    /// Exact path lookup (e.g. `std::sync::Arc` or `E0502`).
    pub fn lookup_symbol(&self, path: &str) -> Option<&DocReference> {
        let clean = path.trim().to_lowercase();
        self.corpus
            .iter()
            .find(|item| item.path.to_lowercase() == clean || item.title.to_lowercase() == clean)
    }

    /// Diagnostic citation lookup for `rustc` compiler error codes (e.g., "E0502", "E0382").
    pub fn cite_for_error(&self, error_code: &str) -> Option<&DocReference> {
        let code = error_code.trim().to_uppercase();
        self.corpus.iter().find(|item| {
            item.category == DocCategory::CompilerError && item.path.to_uppercase() == code
        })
    }

    /// Case-insensitive keyword and token search across titles, paths, summaries, and keywords.
    pub fn search(&self, query: &str, limit: usize) -> Vec<DocReference> {
        let terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if terms.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(usize, &DocReference)> = self
            .corpus
            .iter()
            .filter_map(|doc| {
                let mut score = 0usize;
                let path_lower = doc.path.to_lowercase();
                let title_lower = doc.title.to_lowercase();
                let summary_lower = doc.summary.to_lowercase();

                for term in &terms {
                    if path_lower.contains(term) {
                        score += 10;
                    }
                    if title_lower.contains(term) {
                        score += 8;
                    }
                    if doc.keywords.iter().any(|k| k.to_lowercase() == *term) {
                        score += 6;
                    }
                    if summary_lower.contains(term) {
                        score += 2;
                    }
                }

                if score > 0 {
                    Some((score, doc))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by_key(|a| std::cmp::Reverse(a.0));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, doc)| doc.clone())
            .collect()
    }

    /// Generates a Socratic inquiry response with curated citations for the given query.
    pub fn ask(&self, query: &str) -> LibrarianResponse {
        let matches = self.search(query, 4);
        let primary_doc = matches.first().cloned();
        let related_docs = if matches.len() > 1 {
            matches[1..].to_vec()
        } else {
            Vec::new()
        };

        let socratic_note = if let Some(ref doc) = primary_doc {
            format!(
                "Consider how the ownership and lifetime semantics of `{}` enforce safety invariants in your design: {}. How do these guarantees prevent data races or aliasing bugs?",
                doc.path, doc.summary
            )
        } else {
            "Explore the standard library documentation to identify traits or smart pointer wrappers that guarantee compile-time safety for your concurrency or memory layout requirement.".to_string()
        };

        LibrarianResponse {
            query: query.to_string(),
            primary_doc,
            related_docs,
            socratic_note,
        }
    }

    fn build_canonical_corpus() -> Vec<DocReference> {
        vec![
            // --- std::sync ---
            DocReference {
                title: "Arc (Atomically Reference Counted)".to_string(),
                path: "std::sync::Arc".to_string(),
                category: DocCategory::StdSync,
                summary: "Thread-safe reference-counting pointer. Provides shared ownership of value of type T allocated in heap.".to_string(),
                url: "https://doc.rust-lang.org/std/sync/struct.Arc.html".to_string(),
                code_snippet: Some("use std::sync::Arc;\nlet val = Arc::new(5);\nlet val_clone = Arc::clone(&val);".to_string()),
                keywords: vec!["arc".to_string(), "shared".to_string(), "concurrency".to_string(), "thread".to_string(), "clone".to_string()],
            },
            DocReference {
                title: "Mutex (Mutual Exclusion Primitive)".to_string(),
                path: "std::sync::Mutex".to_string(),
                category: DocCategory::StdSync,
                summary: "Mutual exclusion primitive useful for protecting shared data across threads via RAII MutexGuard locking.".to_string(),
                url: "https://doc.rust-lang.org/std/sync/struct.Mutex.html".to_string(),
                code_snippet: Some("use std::sync::Mutex;\nlet lock = Mutex::new(0);\n*lock.lock().unwrap() += 1;".to_string()),
                keywords: vec!["mutex".to_string(), "lock".to_string(), "exclusive".to_string(), "concurrency".to_string(), "poisoning".to_string()],
            },
            DocReference {
                title: "RwLock (Reader-Writer Lock)".to_string(),
                path: "std::sync::RwLock".to_string(),
                category: DocCategory::StdSync,
                summary: "Reader-writer lock allowing multiple concurrent readers or at most one exclusive writer.".to_string(),
                url: "https://doc.rust-lang.org/std/sync/struct.RwLock.html".to_string(),
                code_snippet: Some("use std::sync::RwLock;\nlet rw = RwLock::new(42);\nlet r = rw.read().unwrap();".to_string()),
                keywords: vec!["rwlock".to_string(), "readers".to_string(), "writers".to_string(), "shared".to_string()],
            },
            DocReference {
                title: "OnceLock (Thread-Safe Lazy Initialization)".to_string(),
                path: "std::sync::OnceLock".to_string(),
                category: DocCategory::StdSync,
                summary: "Synchronization primitive which can be written to only once, providing safe global or static lazy initialization.".to_string(),
                url: "https://doc.rust-lang.org/std/sync/struct.OnceLock.html".to_string(),
                code_snippet: Some("use std::sync::OnceLock;\nstatic CELL: OnceLock<String> = OnceLock::new();\nlet val = CELL.get_or_init(|| \"ready\".to_string());".to_string()),
                keywords: vec!["oncelock".to_string(), "lazy".to_string(), "static".to_string(), "singleton".to_string()],
            },

            // --- std::cell ---
            DocReference {
                title: "RefCell (Dynamic Borrow Checking)".to_string(),
                path: "std::cell::RefCell".to_string(),
                category: DocCategory::StdCell,
                summary: "A mutable memory location with dynamically checked borrow rules at runtime via Ref and RefMut guards.".to_string(),
                url: "https://doc.rust-lang.org/std/cell/struct.RefCell.html".to_string(),
                code_snippet: Some("use std::cell::RefCell;\nlet cell = RefCell::new(10);\n*cell.borrow_mut() += 5;".to_string()),
                keywords: vec!["refcell".to_string(), "interior mutability".to_string(), "borrow".to_string(), "dynamic".to_string()],
            },
            DocReference {
                title: "Cell (Copy Interior Mutability)".to_string(),
                path: "std::cell::Cell".to_string(),
                category: DocCategory::StdCell,
                summary: "A mutable memory location for Copy types that allows mutation without reference borrowing, avoiding runtime borrow check overhead.".to_string(),
                url: "https://doc.rust-lang.org/std/cell/struct.Cell.html".to_string(),
                code_snippet: Some("use std::cell::Cell;\nlet c = Cell::new(100);\nc.set(200);".to_string()),
                keywords: vec!["cell".to_string(), "copy".to_string(), "interior mutability".to_string(), "set".to_string()],
            },

            // --- std::pin ---
            DocReference {
                title: "Pin (Pointers to Immovable Memory)".to_string(),
                path: "std::pin::Pin".to_string(),
                category: DocCategory::StdPin,
                summary: "Pointers that pin their pointees in place, preventing moving values unless they implement Unpin. Essential for self-referential async futures.".to_string(),
                url: "https://doc.rust-lang.org/std/pin/struct.Pin.html".to_string(),
                code_snippet: Some("use std::pin::Pin;\nlet mut boxed = Box::pin(42);\nlet r: Pin<&mut i32> = boxed.as_mut();".to_string()),
                keywords: vec!["pin".to_string(), "unpin".to_string(), "async".to_string(), "future".to_string(), "self-referential".to_string()],
            },

            // --- std::marker ---
            DocReference {
                title: "Send (Safe Cross-Thread Transfer)".to_string(),
                path: "std::marker::Send".to_string(),
                category: DocCategory::StdMarker,
                summary: "Types that can be safely transferred across thread boundaries. Automatically implemented by compiler for types composed entirely of Send fields.".to_string(),
                url: "https://doc.rust-lang.org/std/marker/trait.Send.html".to_string(),
                code_snippet: None,
                keywords: vec!["send".to_string(), "marker".to_string(), "threads".to_string(), "concurrency".to_string()],
            },
            DocReference {
                title: "Sync (Safe Cross-Thread Sharing)".to_string(),
                path: "std::marker::Sync".to_string(),
                category: DocCategory::StdMarker,
                summary: "Types for which it is safe to share references between threads (&T is Send). If T: Sync, concurrent immutable access is data-race-free.".to_string(),
                url: "https://doc.rust-lang.org/std/marker/trait.Sync.html".to_string(),
                code_snippet: None,
                keywords: vec!["sync".to_string(), "marker".to_string(), "sharing".to_string(), "threads".to_string()],
            },
            DocReference {
                title: "PhantomData (Zero-Sized Type Marker)".to_string(),
                path: "std::marker::PhantomData".to_string(),
                category: DocCategory::StdMarker,
                summary: "Zero-sized marker struct used to inform compiler about variance, lifetimes, and unused generic type parameters.".to_string(),
                url: "https://doc.rust-lang.org/std/marker/struct.PhantomData.html".to_string(),
                code_snippet: Some("use std::marker::PhantomData;\nstruct Token<T> { _marker: PhantomData<T> }".to_string()),
                keywords: vec!["phantomdata".to_string(), "variance".to_string(), "generics".to_string(), "subtyping".to_string()],
            },

            // --- Nomicon ---
            DocReference {
                title: "Subtyping and Variance".to_string(),
                path: "nomicon::subtyping".to_string(),
                category: DocCategory::Nomicon,
                summary: "How lifetime relationships dictate subtyping: covariant (&'a T), invariant (&mut T), and contravariant (fn(T)).".to_string(),
                url: "https://doc.rust-lang.org/nomicon/subtyping.html".to_string(),
                code_snippet: None,
                keywords: vec!["variance".to_string(), "subtyping".to_string(), "covariance".to_string(), "invariance".to_string(), "lifetimes".to_string()],
            },
            DocReference {
                title: "Unsafe Rust and Invariants".to_string(),
                path: "nomicon::meet-safe-and-unsafe".to_string(),
                category: DocCategory::Nomicon,
                summary: "The contract between safe and unsafe code: safe abstractions must uphold sound invariants even when wrapping raw pointers.".to_string(),
                url: "https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html".to_string(),
                code_snippet: None,
                keywords: vec!["unsafe".to_string(), "soundness".to_string(), "invariants".to_string(), "raw pointer".to_string()],
            },

            // --- Rust Reference ---
            DocReference {
                title: "Borrow Checker Invariants".to_string(),
                path: "reference::borrow-check".to_string(),
                category: DocCategory::RustReference,
                summary: "Fundamental rule of memory safety: any value can have either one mutable reference (&mut T) or any number of immutable references (&T), never both.".to_string(),
                url: "https://doc.rust-lang.org/reference/expressions.html".to_string(),
                code_snippet: None,
                keywords: vec!["aliasing".to_string(), "borrowing".to_string(), "exclusive".to_string(), "mutable".to_string()],
            },

            // --- Compiler Errors ---
            DocReference {
                title: "E0502: Cannot borrow as mutable because it is also borrowed as immutable".to_string(),
                path: "E0502".to_string(),
                category: DocCategory::CompilerError,
                summary: "Occurs when attempting to create a mutable borrow while an active immutable reference still exists in scope, violating the aliasing invariant.".to_string(),
                url: "https://doc.rust-lang.org/error_codes/E0502.html".to_string(),
                code_snippet: Some("// Solution: Ensure immutable references are dropped before mutable borrow occurs.\nlet mut v = vec![1, 2];\nlet r = &v[0];\n// v.push(3); // Error: E0502\nprintln!(\"{r}\");\nv.push(3); // OK: r dropped".to_string()),
                keywords: vec!["e0502".to_string(), "borrow".to_string(), "mutable".to_string(), "aliasing".to_string()],
            },
            DocReference {
                title: "E0382: Use of moved value".to_string(),
                path: "E0382".to_string(),
                category: DocCategory::CompilerError,
                summary: "Occurs when accessing a variable after its ownership has been transferred to another scope, closure, or function.".to_string(),
                url: "https://doc.rust-lang.org/error_codes/E0382.html".to_string(),
                code_snippet: Some("let s = String::from(\"hello\");\nlet s2 = s; // moved\n// println!(\"{s}\"); // Error: E0382\nprintln!(\"{s2}\"); // OK".to_string()),
                keywords: vec!["e0382".to_string(), "move".to_string(), "ownership".to_string(), "transfer".to_string()],
            },
            DocReference {
                title: "E0597: Value does not live long enough".to_string(),
                path: "E0597".to_string(),
                category: DocCategory::CompilerError,
                summary: "Occurs when a reference outlives the data it points to, creating a dangling pointer risk prevented by the borrow checker.".to_string(),
                url: "https://doc.rust-lang.org/error_codes/E0597.html".to_string(),
                code_snippet: None,
                keywords: vec!["e0597".to_string(), "lifetime".to_string(), "dangling".to_string(), "scope".to_string()],
            },
            DocReference {
                title: "E0277: Trait is not implemented for type".to_string(),
                path: "E0277".to_string(),
                category: DocCategory::CompilerError,
                summary: "Occurs when a type does not satisfy the trait bounds required by a generic function or trait implementation.".to_string(),
                url: "https://doc.rust-lang.org/error_codes/E0277.html".to_string(),
                code_snippet: None,
                keywords: vec!["e0277".to_string(), "trait".to_string(), "bound".to_string(), "unimplemented".to_string()],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_symbol_exact_and_case_insensitive() {
        let librarian = LibrarianAgent::new();
        let doc = librarian.lookup_symbol("std::sync::Arc");
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.title, "Arc (Atomically Reference Counted)");
        assert_eq!(doc.category, DocCategory::StdSync);

        // Case-insensitive lookup
        let doc_lower = librarian.lookup_symbol("std::sync::arc");
        assert!(doc_lower.is_some());
    }

    #[test]
    fn test_cite_for_compiler_error() {
        let librarian = LibrarianAgent::new();
        let e0502 = librarian.cite_for_error("e0502");
        assert!(e0502.is_some());
        let doc = e0502.unwrap();
        assert_eq!(doc.category, DocCategory::CompilerError);
        assert!(doc.summary.contains("aliasing invariant"));

        let unknown = librarian.cite_for_error("E9999");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_keyword_search_ranking() {
        let librarian = LibrarianAgent::new();
        let results = librarian.search("threads mutex", 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "std::sync::Mutex");
    }

    #[test]
    fn test_ask_generates_socratic_note() {
        let librarian = LibrarianAgent::new();
        let resp = librarian.ask("Tell me about Pin in async");
        assert!(resp.primary_doc.is_some());
        let primary = resp.primary_doc.unwrap();
        assert_eq!(primary.path, "std::pin::Pin");
        assert!(resp.socratic_note.contains("invariants"));
    }
}
