//! Tutor Agent: Generates concise, high-density theory and dynamic compiler error revisions.
//!
//! Delivers pedagogical materials with concrete code examples and generates targeted revisions
//! specialized for rustc compiler diagnostics (E0382, E0597, E0502, E0277, E0499, E0308, E0716).

use async_trait::async_trait;
use nereus_core::models::RoadmapTopic;
use nereus_core::traits::Agent;
use serde::{Deserialize, Serialize};

/// Diagnostic metadata and pedagogical remediation for known rustc compiler error codes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerDiagnosticGuide {
    pub code: &'static str,
    pub title: &'static str,
    pub concept: &'static str,
    pub anti_pattern: &'static str,
    pub idiomatic_solution: &'static str,
    pub socratic_prompt: &'static str,
}

/// Catalog of rustc compiler errors with remediation strategies.
pub static COMPILER_GUIDES: &[CompilerDiagnosticGuide] = &[
    CompilerDiagnosticGuide {
        code: "E0382",
        title: "Use of Moved Value (Move Semantics)",
        concept: "Rust types not implementing `Copy` are moved on assignment or by-value call, invalidating the original binding.",
        anti_pattern: r#"let s = String::from("nereus");
let handle = std::thread::spawn(move || {
    println!("{s}");
});
println!("{s}"); // ERROR E0382: borrow of moved value: `s`"#,
        idiomatic_solution: r#"// Solution 1: Use an atomic reference counter (Arc) for shared ownership across threads
let s = std::sync::Arc::new(String::from("nereus"));
let s_clone = std::sync::Arc::clone(&s);
let handle = std::thread::spawn(move || {
    println!("{s_clone}");
});
println!("{s}"); // Valid: `s` remains accessible"#,
        socratic_prompt: "Who is the exclusive owner of this value at this point in the control flow? How does `Arc::clone` or passing a shared borrow `&T` preserve access without moving ownership?",
    },
    CompilerDiagnosticGuide {
        code: "E0597",
        title: "Borrowed Value Does Not Live Long Enough",
        concept: "A reference cannot outlive the lifetime scope of the underlying owner whose data it borrows.",
        anti_pattern: r#"let r: &String;
{
    let local = String::from("transient");
    r = &local;
} // `local` dropped here
println!("{r}"); // ERROR E0597: `local` does not live long enough"#,
        idiomatic_solution: r#"// Solution: Ensure the owner lives at least as long as all references borrowing from it
let owner = String::from("persistent");
let r = &owner;
println!("{r}"); // Valid: `owner` outlives reference `r`"#,
        socratic_prompt: "Compare the lexical lifetime of the referenced variable with the lifetime of the reference. Where is the owner dropped relative to the reference's last read?",
    },
    CompilerDiagnosticGuide {
        code: "E0502",
        title: "Cannot Borrow as Mutable While Also Borrowed as Immutable",
        concept: "The Aliasing XOR Mutability invariant forbids concurrent read and write access to the same memory location.",
        anti_pattern: r#"let mut vec = vec![1, 2, 3];
let first = &vec[0]; // Immutable borrow begins
vec.push(4);         // ERROR E0502: cannot borrow `vec` as mutable because it is also borrowed as immutable
println!("{first}"); // Immutable borrow used here"#,
        idiomatic_solution: r#"// Solution: Scope or copy the read borrow before initiating mutable re-allocation
let mut vec = vec![1, 2, 3];
let first_val = vec[0]; // Copy the primitive value or drop reference before mutation
vec.push(4);
println!("{first_val}"); // Valid: no active borrows across mutation"#,
        socratic_prompt: "Why does re-allocating or mutating a collection invalidate existing references to its interior elements? How can you decouple the read from the mutation?",
    },
    CompilerDiagnosticGuide {
        code: "E0277",
        title: "Trait Bound Not Satisfied",
        concept: "The generic type parameter or variable does not satisfy a required trait contract, such as Send, Sync, or Clone.",
        anti_pattern: r#"use std::rc::Rc;
let val = Rc::new(42);
tokio::spawn(async move {
    // ERROR E0277: `Rc<i32>` cannot be sent between threads safely (`Send` is not satisfied)
    println!("{val}");
});"#,
        idiomatic_solution: r#"use std::sync::Arc;
let val = Arc::new(42);
tokio::spawn(async move {
    // Valid: `Arc<T>` implements `Send + Sync` when T: Send + Sync
    println!("{val}");
});"#,
        socratic_prompt: "What guarantees does the missing trait enforce? For thread boundaries (`Send`/`Sync`), why is non-atomic reference counting (`Rc`) unsafe across concurrent threads?",
    },
    CompilerDiagnosticGuide {
        code: "E0499",
        title: "Cannot Borrow as Mutable More Than Once at a Time",
        concept: "Rust permits exactly one active mutable reference to any data structure within a given scope.",
        anti_pattern: r#"let mut data = vec![1, 2, 3];
let ref1 = &mut data;
let ref2 = &mut data; // ERROR E0499: cannot borrow `data` as mutable more than once at a time
ref1.push(4);
ref2.push(5);"#,
        idiomatic_solution: r#"// Solution: Restructure into sequential exclusive scopes or split borrows across fields
let mut data = vec![1, 2, 3];
{
    let ref1 = &mut data;
    ref1.push(4);
} // ref1 scope ends here
let ref2 = &mut data;
ref2.push(5); // Valid: sequential exclusive borrows"#,
        socratic_prompt: "How does having two simultaneous mutable references create a data race even in single-threaded code? Can the operations be serialized into distinct scopes?",
    },
    CompilerDiagnosticGuide {
        code: "E0308",
        title: "Mismatched Types",
        concept: "An expression evaluates to a type differing from the expected type signature in the declaration or function return.",
        anti_pattern: r#"fn parse_id(val: Option<&str>) -> Result<u64, &'static str> {
    val.map(|s| s.parse::<u64>().unwrap()) // ERROR E0308: expected `Result`, found `Option`
}"#,
        idiomatic_solution: r#"fn parse_id(val: Option<&str>) -> Result<u64, &'static str> {
    val.ok_or("Missing ID")
        .and_then(|s| s.parse::<u64>().map_err(|_| "Malformed integer"))
}"#,
        socratic_prompt: "What combinator transforms an `Option<T>` into a `Result<T, E>`? Are you propagating error states with the `?` operator or transforming error types via `map_err`?",
    },
    CompilerDiagnosticGuide {
        code: "E0716",
        title: "Temporary Value Dropped While Borrowed",
        concept: "A reference was taken to a temporary intermediate expression that is dropped at the end of the statement.",
        anti_pattern: r#"let ref_str: &str = &String::from("temporary").trim();
// ERROR E0716: temporary value dropped while borrowed"#,
        idiomatic_solution: r#"// Solution: Bind the temporary owned value to a local variable to extend its lifetime
let owned_str = String::from("temporary");
let ref_str: &str = owned_str.trim();
println!("{ref_str}"); // Valid: owned_str outlives ref_str"#,
        socratic_prompt: "What is the lifetime of a temporary value created within an expression? How does assigning it to an explicit `let` binding extend its lifetime to the enclosing block?",
    },
];

/// Tutor Agent delivering high-density theory and compiler-error-driven revisions.
#[derive(Debug, Default)]
pub struct TutorAgent;

impl TutorAgent {
    pub fn new() -> Self {
        Self
    }

    /// Locate diagnostic guide for a specific compiler error code.
    pub fn lookup_compiler_guide(code: &str) -> Option<&'static CompilerDiagnosticGuide> {
        let code_upper = code.to_uppercase();
        COMPILER_GUIDES.iter().find(|g| code_upper.contains(g.code))
    }

    /// Extract compiler error code from raw error text if present.
    pub fn extract_compiler_error_code(text: &str) -> Option<&'static str> {
        for guide in COMPILER_GUIDES {
            if text.contains(guide.code) {
                return Some(guide.code);
            }
        }
        None
    }

    /// Generate concise, high-density pedagogical theory material with code examples and challenge task.
    pub fn provide_material(&self, topic: &RoadmapTopic) -> (String, String) {
        let (invariants, code_example, pitfall) = self.generate_topic_content(topic);

        let material = format!(
            r#"# {title}

**Curriculum Topic**: `{id}` | **Difficulty**: `{diff:.1}/1.0` | **Est. Time**: `{hours:.1}h`

## Core Architecture & Invariants
{desc}

{invariants}

## Idiomatic Rust Pattern
```rust
{code_example}
```

## Critical Pitfalls & Anti-Patterns
{pitfall}

## Socratic Mental Model Checklist
- [ ] Memory layout: Is data on the stack, heap, or read-only text segment?
- [ ] Ownership flow: Who allocates, borrows, mutates, and drops the resource?
- [ ] Concurrency safety: Does this type satisfy `Send + Sync` invariants?
- [ ] Failure paths: Are all errors handled via typed `Result` or `?` propagation?
"#,
            title = topic.title,
            id = topic.id,
            diff = topic.difficulty,
            hours = topic.estimated_hours,
            desc = topic.description,
            invariants = invariants,
            code_example = code_example,
            pitfall = pitfall,
        );

        let task = format!(
            "Implement a production-grade, thread-safe solution for «{}».\n\
            Requirements:\n\
            1. Enforce strict type invariants and zero undefined behavior.\n\
            2. Pass all unit assertions, concurrency stress tests, and clippy lints.\n\
            3. Gracefully propagate error conditions without unhandled panics.",
            topic.title
        );

        (material, task)
    }

    /// Generate dynamic revision material specialized for identified weak areas or compiler errors.
    pub fn provide_revision(
        &self,
        topic: &RoadmapTopic,
        weak_areas: &[String],
    ) -> (String, String) {
        // Check if any weak area contains a known compiler error code
        for weak in weak_areas {
            if let Some(code) = Self::extract_compiler_error_code(weak) {
                return self.provide_revision_for_compiler_error(topic, code);
            }
        }

        // Generic multi-area revision
        let weak_summary = if weak_areas.is_empty() {
            "General borrow checker and type system invariants".to_string()
        } else {
            weak_areas.join(", ")
        };

        let material = format!(
            r#"# Targeted Revision: {title}

**Focus Areas**: {weak_summary}

## Diagnostic Refactoring Guidance
Your prior implementation encountered friction with Rust's compiler verification gates.
Review the following mental model to unblock your design:

### 1. Aliasing XOR Mutability
You may have multiple shared references (`&T`) OR a single exclusive reference (`&mut T`), never both.
When you need shared mutable state across threads:
```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let state = Arc::new(Mutex::new(0));
let state_worker = Arc::clone(&state);
tokio::spawn(async move {{
    let mut lock = state_worker.lock().await;
    *lock += 1;
}});
```

### 2. Error Propagation Contract
Avoid `.unwrap()` or `.expect()` in production modules. Compose results explicitly:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {{
    #[error("I/O failure: {{0}}")]
    Io(#[from] std::io::Error),
    #[error("Validation failed: {{0}}")]
    InvalidInput(String),
}}
```
"#,
            title = topic.title,
            weak_summary = weak_summary,
        );

        let task = format!(
            "Refactor your solution for «{}» to address gaps in: {}.\n\
            Ensure memory safety invariants are satisfied before re-submitting.",
            topic.title, weak_summary
        );

        (material, task)
    }

    /// Generate specialized revision material tailored to a specific compiler error code.
    pub fn provide_revision_for_compiler_error(
        &self,
        topic: &RoadmapTopic,
        error_text: &str,
    ) -> (String, String) {
        if let Some(guide) = Self::lookup_compiler_guide(error_text) {
            let material = format!(
                r#"# Revision: {title} — Compiler Diagnostic [{code}]

### Diagnostic: {guide_title}
**Inherent Invariant**: {concept}

---

### Anti-Pattern (What Failed)
```rust
{anti_pattern}
```

### Idiomatic Resolution Pattern
```rust
{solution}
```

---

### Socratic Reflection
> {prompt}
"#,
                title = topic.title,
                code = guide.code,
                guide_title = guide.title,
                concept = guide.concept,
                anti_pattern = guide.anti_pattern,
                solution = guide.idiomatic_solution,
                prompt = guide.socratic_prompt,
            );

            let task = format!(
                "Refactor the implementation of «{}» to resolve compiler diagnostic [{}: {}].\n\
                Apply the idiomatic resolution pattern and verify all compiler errors are eliminated.",
                topic.title, guide.code, guide.title
            );

            (material, task)
        } else {
            self.provide_revision(topic, &[error_text.to_string()])
        }
    }

    /// Generate domain-specific theory content for a topic.
    fn generate_topic_content(&self, topic: &RoadmapTopic) -> (String, String, String) {
        let id = topic.id.as_str();

        if id.contains("ownership") {
            (
                "### Invariants\n- Each value in Rust has exactly one owner.\n- When the owner goes out of scope, the value is dropped.\n- Assignment of non-Copy types moves ownership.".to_string(),
                r#"struct Resource {
    id: u64,
}

impl Drop for Resource {
    fn drop(&mut self) {
        // Deterministic cleanup
    }
}

fn consume(r: Resource) {
    // r takes ownership and drops here
}

fn main() {
    let r = Resource { id: 1 };
    consume(r); // r moved
    // r is no longer valid here
}"#.to_string(),
                "**Pitfall**: Attempting to use a binding after passing it by value to a function or storing it in a collection.".to_string(),
            )
        } else if id.contains("borrow") {
            (
                "### Invariants\n- Any borrow must last for a scope no greater than that of the owner.\n- You may have any number of `&T` or exactly one `&mut T`.".to_string(),
                r#"let mut data = vec![1, 2, 3];
// Immutable borrows
let r1 = &data;
let r2 = &data;
println!("{r1:?}, {r2:?}"); // r1, r2 end here under NLL

// Mutable borrow starts safely afterwards
let r3 = &mut data;
r3.push(4);"#.to_string(),
                "**Pitfall**: Mutating a vector or map while holding a reference to one of its elements.".to_string(),
            )
        } else if id.contains("lifetime") {
            (
                "### Invariants\n- Generic lifetime parameters represent constraints, not runtime timers.\n- Lifetime elision automatically inserts lifetimes for common patterns.\n- Structs containing references must declare their lifetime parameters.".to_string(),
                r#"struct Excerpt<'a> {
    part: &'a str,
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}"#.to_string(),
                "**Pitfall**: Returning a reference to a local variable created inside the function body.".to_string(),
            )
        } else if id.contains("tokio") || id.contains("async") {
            (
                "### Invariants\n- Async tasks must yield cooperatively via `.await`.\n- Worker threads must never execute blocking CPU or I/O loops.\n- State sharing across tasks requires `Arc<Mutex<T>>` or channels.".to_string(),
                r#"use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<String>(32);

    tokio::spawn(async move {
        tx.send("message".to_string()).await.unwrap();
    });

    while let Some(msg) = rx.recv().await {
        println!("Received: {msg}");
    }
}"#.to_string(),
                "**Pitfall**: Holding a `std::sync::MutexGuard` across an `.await` point, causing deadlock across async threads.".to_string(),
            )
        } else if id.contains("error") {
            (
                "### Invariants\n- Recoverable errors must use `Result<T, E>`.\n- Custom error enums should implement `std::error::Error` via `thiserror`.\n- Propagate errors cleanly using `?`.".to_string(),
                r#"use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Configuration missing key: {0}")]
    MissingKey(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn run_pipeline() -> Result<(), PipelineError> {
    // Pipeline execution logic with ? operator
    Ok(())
}"#.to_string(),
                "**Pitfall**: Using stringly-typed errors (`Result<T, String>`) in public library APIs instead of structured enums.".to_string(),
            )
        } else {
            (
                "### Invariants\n- Traits define shared abstract behavior.\n- Generic trait bounds enable zero-cost compile-time static dispatch.\n- Trait objects (`dyn Trait`) enable runtime polymorphism with vtables.".to_string(),
                r#"pub trait Worker: Send + Sync {
    fn execute(&self) -> Result<(), &'static str>;
}

pub fn run_worker<W: Worker>(worker: &W) {
    worker.execute().ok();
}"#.to_string(),
                "**Pitfall**: Attempting to create a trait object `dyn Trait` for a trait that is not object-safe (e.g., has methods with generic type parameters).".to_string(),
            )
        }
    }
}

#[async_trait]
impl Agent for TutorAgent {
    fn id(&self) -> &'static str {
        "tutor-agent"
    }

    fn name(&self) -> &'static str {
        "Tutor Agent"
    }
}
