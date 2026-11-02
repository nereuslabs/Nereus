# Nereus 2.0 Technical Specification & Architecture Design

## 1. System Philosophy

Nereus 2.0 is an interactive coding cockpit and autonomous AI tutor written in **idiomatic Rust**.
The system replaces opaque, heavyweight Python frameworks (LangGraph, Chainlit, Pydantic) with:
1. **Compile-Time State Verification**: An algebraic finite state machine (FSM) where invalid states are unrepresentable.
2. **Sub-millisecond Latency & Single Binary Distribution**: Zero runtime dependencies, instant startup (<5ms).
3. **Execution-Driven Assessment**: Code is compiled and tested inside an isolated sandbox before evaluation.
4. **Hermetic Testability**: Complete determinism in test environments via a `StubLLMClient`.

---

## 2. Technology Stack & Crates

```toml
[package]
name = "nereus"
version = "0.2.0"
edition = "2021"
license = "MIT"
authors = ["Nereus Labs <develop.nereus@gmail.com>"]

[dependencies]
# Async Runtime & Concurrency
tokio = { version = "1.38", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Terminal UI & CLI
ratatui = "0.26"
crossterm = "0.27"
clap = { version = "4.5", features = ["derive", "env"] }

# Serialization & Models
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Networking & LLM API
reqwest = { version = "0.12", features = ["json", "stream"] }
eventsource-stream = "0.2"

# Persistence & Storage
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1.8", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Logging & Error Handling
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "1.0"
anyhow = "1.0"
```

---

## 3. The Algebraic State Machine (Replacing LangGraph)

In Nereus 1.x, LangGraph maintained a generic dictionary state with Python runtime interrupts.
In Nereus 2.0, the state is modeled as an explicit Rust algebraic enum:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningState {
    /// Initial phase: diagnosing learner's baseline.
    Diagnostic {
        questions: Vec<DiagnosticQuestion>,
        current_index: usize,
        answers: Vec<String>,
    },
    /// Generating prioritized roadmap based on weaknesses.
    Coaching {
        weakness_report: WeaknessReport,
    },
    /// Delivering conceptual theory and code exercise.
    Tutoring {
        topic: RoadmapTopic,
        material: String,
        task: String,
        revision_count: usize,
    },
    /// Waiting for user submission and running test suite.
    Examining {
        topic: RoadmapTopic,
        task: String,
        submission: Option<String>,
        assessment: Option<Assessment>,
    },
    /// All topics in the roadmap mastered.
    Completed,
}

pub struct StateMachineContext {
    pub session_id: uuid::Uuid,
    pub profile: UserProfile,
    pub roadmap: Roadmap,
    pub current_topic_index: usize,
    pub state: LearningState,
}
```

---

## 4. The 4 Autonomous Agents

### 1. `DiagnosticAgent`
* **Trigger**: Invoked when a new session starts (or `--diagnostic` flag is passed).
* **Duty**: Asks 3–5 targeted questions covering core concepts, syntax, and error handling.
* **Output**: `WeaknessReport { weak_areas: Vec<String>, recommended_focus: Vec<String> }`.

### 2. `CoachAgent`
* **Duty**: Generates or adjusts a ordered `Roadmap` of `RoadmapTopic`s.
* **Input**: User goal, daily hours, deadline, and diagnostic `WeaknessReport`.
* **Output**: `Roadmap { topics: Vec<RoadmapTopic> }` where topics are sorted by dependencies and weak areas first.

### 3. `TutorAgent`
* **Duty**: Prepares bite-sized markdown materials and practical coding tasks.
* **Special Handling**: On a `RETRY` verdict, dynamically generates revision material addressing the specific compiler or conceptual errors reported by the Examiner.

### 4. `ExaminerAgent`
* **Duty**: Runs student code in sandbox $\rightarrow$ inspects compiler output / test assertions $\rightarrow$ generates structured `AssessmentOutput`.
* **Threshold**: Dynamic passing score formula: `70.0 + (topic.difficulty * 15.0)` (clamped $\le 85.0$).
* **Verdict**: `Pass` (advances to next topic) or `Retry` (invokes Socratic hints).

---

## 5. Offline-First Architecture & Testing

To ensure zero API costs during development and hermetic CI pipelines:
```rust
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, LLMError>;
    async fn structured_output<T: serde::de::DeserializeOwned>(&self, prompt: &str) -> Result<T, LLMError>;
}

pub struct StubLLMClient;

#[async_trait]
impl LLMClient for StubLLMClient {
    async fn complete(&self, _prompt: &str) -> Result<String, LLMError> {
        Ok("Stub response: topic mastered.".to_string())
    }
    // Returns deterministic mock instances of Roadmap, Assessment, etc.
}
```

---

## 6. Socratic Feedback Engine

Instead of simply outputting the corrected code, the Examiner adheres to the Socratic prompting rule:
1. Identify the failing assertion or compiler error.
2. Ask a guided question that leads the learner to discover the root cause.
3. Provide one conceptual hint without giving away the direct code syntax.
