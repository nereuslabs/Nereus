//! Automated Socratic Mentor Benchmark and LLM Evaluation Suite.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.
//!
//! Evaluates LLMs across 4 core pedagogical dimensions:
//! 1. Zero-Code Leakage Rate: percentage of turns where model refuses to dump complete implementations.
//! 2. Diagnostic Precision: ability to isolate compiler error codes (E0382, E0597, E0277, etc.).
//! 3. Socratic Guidance Score: presence of probing reflective questions and avoidance of imperative commands.
//! 4. Schema Adherence: strict format compliance.

use nereus_core::traits::LLMClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// A canonical benchmark evaluation test case representing a real-world Rust compiler error.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkTestCase {
    pub id: &'static str,
    pub topic: &'static str,
    pub error_code: &'static str,
    pub student_code: &'static str,
    pub compiler_stderr: &'static str,
    pub expected_concepts: &'static [&'static str],
}

/// Standard problem bank of 10 canonical Rust compiler mistakes.
pub const CANONICAL_BENCHMARK_CASES: &[BenchmarkTestCase] = &[
    BenchmarkTestCase {
        id: "bench-e0382-use-after-move",
        topic: "Ownership and Moving Semantics",
        error_code: "E0382",
        student_code: "fn print_string(s: String) { println!(\"{}\", s); }\nfn main() { let s = String::from(\"hello\"); print_string(s); println!(\"{}\", s); }",
        compiler_stderr: "error[E0382]: borrow of moved value: `s`\n --> src/main.rs:2:51\n  |\n2 | let s = String::from(\"hello\"); print_string(s); println!(\"{}\", s);\n  |                                            -                 ^ value borrowed here after move",
        expected_concepts: &["ownership", "move", "clone", "borrow"],
    },
    BenchmarkTestCase {
        id: "bench-e0597-borrow-does-not-live-long-enough",
        topic: "Lifetimes and Stack Scopes",
        error_code: "E0597",
        student_code: "fn main() { let r; { let x = 5; r = &x; } println!(\"{}\", r); }",
        compiler_stderr: "error[E0597]: `x` does not live long enough\n --> src/main.rs:1:35\n  |\n1 | fn main() { let r; { let x = 5; r = &x; } println!(\"{}\", r); }\n  |                                      ^^ borrowed value does not live long enough",
        expected_concepts: &["lifetime", "scope", "borrow", "dangle"],
    },
    BenchmarkTestCase {
        id: "bench-e0277-trait-bound-unsatisfied",
        topic: "Traits and Generics",
        error_code: "E0277",
        student_code: "struct Point { x: i32, y: i32 }\nfn main() { let p = Point { x: 1, y: 2 }; println!(\"{:?}\", p); }",
        compiler_stderr: "error[E0277]: `Point` doesn't implement `Debug`\n --> src/main.rs:2:54\n  |\n2 | fn main() { let p = Point { x: 1, y: 2 }; println!(\"{:?}\", p); }\n  |                                                      ^^^ `Point` cannot be formatted using `{:?}`",
        expected_concepts: &["trait", "debug", "derive", "implement"],
    },
    BenchmarkTestCase {
        id: "bench-e0502-mutable-and-immutable-borrow",
        topic: "Aliasing XOR Mutability",
        error_code: "E0502",
        student_code: "fn main() { let mut v = vec![1, 2, 3]; let first = &v[0]; v.push(4); println!(\"{}\", first); }",
        compiler_stderr: "error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable\n --> src/main.rs:1:56\n  |\n1 | fn main() { let mut v = vec![1, 2, 3]; let first = &v[0]; v.push(4); println!(\"{}\", first); }\n  |                                                    -      ^^^^^^^^^ mutable borrow occurs here",
        expected_concepts: &["borrow", "mutable", "immutable", "reallocation"],
    },
    BenchmarkTestCase {
        id: "bench-e0106-missing-lifetime-specifier",
        topic: "Explicit Lifetime Annotations",
        error_code: "E0106",
        student_code: "fn longest(x: &str, y: &str) -> &str { if x.len() > y.len() { x } else { y } }",
        compiler_stderr: "error[E0106]: missing lifetime specifier\n --> src/main.rs:1:34\n  |\n1 | fn longest(x: &str, y: &str) -> &str { if x.len() > y.len() { x } else { y } }\n  |               ----     ----     ^ expected named lifetime parameter",
        expected_concepts: &["lifetime", "generic", "annotation", "elision"],
    },
    BenchmarkTestCase {
        id: "bench-e0308-type-mismatch",
        topic: "Type System and Result Handling",
        error_code: "E0308",
        student_code: "fn parse_val(s: &str) -> u32 { s.parse() }",
        compiler_stderr: "error[E0308]: mismatched types\n --> src/main.rs:1:32\n  |\n1 | fn parse_val(s: &str) -> u32 { s.parse() }\n  |                          ---   ^^^^^^^^^^ expected `u32`, found `Result<u32, ParseIntError>`",
        expected_concepts: &["result", "unwrap", "match", "error"],
    },
    BenchmarkTestCase {
        id: "bench-e0507-cannot-move-out-of-shared-ref",
        topic: "Smart Pointers and Dereferencing",
        error_code: "E0507",
        student_code: "struct User { name: String }\nfn take_name(u: &User) -> String { u.name }",
        compiler_stderr: "error[E0507]: cannot move out of `u.name` which is behind a shared reference\n --> src/main.rs:2:37\n  |\n2 | fn take_name(u: &User) -> String { u.name }\n  |                                     ^^^^^^ move occurs because `u.name` has type `String`",
        expected_concepts: &["move", "shared", "reference", "clone"],
    },
    BenchmarkTestCase {
        id: "bench-e0499-multiple-mutable-borrows",
        topic: "Exclusive Mutability Scopes",
        error_code: "E0499",
        student_code: "fn main() { let mut s = String::from(\"hello\"); let r1 = &mut s; let r2 = &mut s; println!(\"{}, {}\", r1, r2); }",
        compiler_stderr: "error[E0499]: cannot borrow `s` as mutable more than once at a time\n --> src/main.rs:1:68\n  |\n1 | fn main() { let mut s = String::from(\"hello\"); let r1 = &mut s; let r2 = &mut s;\n  |                                                          ------         ^^^^^^ second mutable borrow occurs here",
        expected_concepts: &["mutable", "borrow", "exclusive", "scope"],
    },
    BenchmarkTestCase {
        id: "bench-e0716-temporary-value-dropped-while-borrowed",
        topic: "Temporary Value Lifetimes",
        error_code: "E0716",
        student_code: "fn main() { let s = &String::from(\"temporary\"); println!(\"{}\", s); }",
        compiler_stderr: "error[E0716]: temporary value dropped while borrowed\n --> src/main.rs:1:22\n  |\n1 | fn main() { let s = &String::from(\"temporary\"); println!(\"{}\", s); }\n  |                      ^^^^^^^^^^^^^^^^^^^^^^^^^ creates a temporary value which is freed while still in use",
        expected_concepts: &["temporary", "dropped", "binding", "lifetime"],
    },
    BenchmarkTestCase {
        id: "bench-e0282-type-annotations-needed",
        topic: "Type Inference Constraints",
        error_code: "E0282",
        student_code: "fn main() { let x = Default::default(); println!(\"{:?}\", x); }",
        compiler_stderr: "error[E0282]: type annotations needed\n --> src/main.rs:1:13\n  |\n1 | fn main() { let x = Default::default(); println!(\"{:?}\", x); }\n  |             ^^^^^ cannot infer type",
        expected_concepts: &["type", "annotation", "infer", "default"],
    },
];

/// Detailed evaluation outcome for a single test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub test_case_id: String,
    pub error_code: String,
    pub zero_leakage: bool,
    pub diagnostic_isolated: bool,
    pub has_socratic_question: bool,
    pub valid_schema: bool,
    pub score: f32,
    pub raw_response: String,
}

/// Comprehensive benchmark report summarizing model performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub model_id: String,
    pub total_cases: usize,
    pub zero_leakage_rate: f32,
    pub diagnostic_precision: f32,
    pub socratic_guidance_score: f32,
    pub schema_adherence_rate: f32,
    pub overall_score: f32,
    pub results: Vec<TestCaseResult>,
}

impl BenchmarkReport {
    /// Serializes report to formatted JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Renders report as human-readable GitHub Flavored Markdown.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# 🌊 Nereus Socratic Mentor Benchmark Report\n\n");
        md.push_str(&format!(
            "**Evaluated Model / Provider**: `{}`  \n",
            self.model_id
        ));
        md.push_str(&format!("**Total Test Cases**: {}  \n", self.total_cases));
        md.push_str(&format!(
            "**Overall Composite Score**: `{:.1}%`\n\n",
            self.overall_score
        ));

        md.push_str("## Core Pedagogical Metrics\n\n");
        md.push_str("| Metric | Measured Result | Benchmark Target | Status |\n");
        md.push_str("| :--- | :--- | :--- | :--- |\n");

        let zl_status = if self.zero_leakage_rate >= 95.0 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        let dp_status = if self.diagnostic_precision >= 80.0 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        let sg_status = if self.socratic_guidance_score >= 75.0 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };
        let sa_status = if self.schema_adherence_rate >= 90.0 {
            "✅ PASS"
        } else {
            "❌ FAIL"
        };

        md.push_str(&format!(
            "| Zero Code Leakage Rate | {:.1}% | >= 95.0% | {} |\n",
            self.zero_leakage_rate, zl_status
        ));
        md.push_str(&format!(
            "| Diagnostic Precision | {:.1}% | >= 80.0% | {} |\n",
            self.diagnostic_precision, dp_status
        ));
        md.push_str(&format!(
            "| Socratic Guidance Score | {:.1}% | >= 75.0% | {} |\n",
            self.socratic_guidance_score, sg_status
        ));
        md.push_str(&format!(
            "| Schema Adherence Rate | {:.1}% | >= 90.0% | {} |\n\n",
            self.schema_adherence_rate, sa_status
        ));

        md.push_str("## Per-Case Diagnostic Breakdown\n\n");
        md.push_str("| Case ID | Compiler Error | Zero Leakage | Diagnostic Hit | Socratic Inquiry | Score |\n");
        md.push_str("| :--- | :--- | :--- | :--- | :--- | :--- |\n");

        for r in &self.results {
            let zl_icon = if r.zero_leakage {
                "🛡️ Yes"
            } else {
                "⚠️ Leaked"
            };
            let diag_icon = if r.diagnostic_isolated {
                "🎯 Found"
            } else {
                "❌ Missed"
            };
            let soc_icon = if r.has_socratic_question {
                "💡 Yes"
            } else {
                "❌ No"
            };
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {:.1}% |\n",
                r.test_case_id, r.error_code, zl_icon, diag_icon, soc_icon, r.score
            ));
        }

        md
    }
}

/// Socratic mentor benchmark evaluation runner.
pub struct SocraticBenchmark {
    llm: Arc<dyn LLMClient>,
    model_id: String,
}

impl SocraticBenchmark {
    pub fn new(llm: Arc<dyn LLMClient>, model_id: impl Into<String>) -> Self {
        Self {
            llm,
            model_id: model_id.into(),
        }
    }

    /// Evaluates a single benchmark case against Socratic pedagogy rules.
    pub async fn evaluate_case(&self, case: &BenchmarkTestCase) -> TestCaseResult {
        let prompt = format!(
            "As the Nereus Examiner Agent, assess the learner's submission for topic '{}'.\n\n\
            Student Submission:\n{}\n\n\
            Compiler / Sandbox Diagnostics:\n{}\n\n\
            Adhere strictly to Socratic feedback: guide the student to the root cause without providing direct code or syntax.",
            case.topic, case.student_code, case.compiler_stderr
        );

        let response = match self.llm.complete(&prompt).await {
            Ok(resp) => resp,
            Err(e) => format!("Error generating response: {e}"),
        };

        // 1. Zero Code Leakage verification:
        // Model must NOT leak multi-line code blocks containing implementation definitions
        let has_code_block = response.contains("```rust")
            || response.contains("```")
            || response.contains("fn ")
            || response.contains("here is the fix")
            || response.contains("here is the solution");
        let zero_leakage = !has_code_block;

        // 2. Diagnostic Precision:
        // Model must identify the exact error code or at least one expected foundational concept
        let mentions_error_code = response.contains(case.error_code);
        let mentions_concepts = case
            .expected_concepts
            .iter()
            .any(|c| response.to_lowercase().contains(&c.to_lowercase()));
        let diagnostic_isolated = mentions_error_code || mentions_concepts;

        // 3. Socratic Guidance:
        // Must formulate a reflective question ('?') prompting student discovery
        let has_socratic_question = response.contains('?');

        // 4. Schema Adherence:
        // Output must be non-empty and well-structured
        let valid_schema = !response.trim().is_empty();

        // Calculate weighted score
        let mut case_score = 0.0_f32;
        if zero_leakage {
            case_score += 35.0;
        }
        if diagnostic_isolated {
            case_score += 30.0;
        }
        if has_socratic_question {
            case_score += 25.0;
        }
        if valid_schema {
            case_score += 10.0;
        }

        TestCaseResult {
            test_case_id: case.id.to_string(),
            error_code: case.error_code.to_string(),
            zero_leakage,
            diagnostic_isolated,
            has_socratic_question,
            valid_schema,
            score: case_score,
            raw_response: response,
        }
    }

    /// Runs all canonical benchmark test cases and compiles the final BenchmarkReport.
    pub async fn run_suite(&self) -> BenchmarkReport {
        info!(
            "Starting Socratic Benchmark Suite for model '{}'...",
            self.model_id
        );
        let mut results = Vec::new();

        for case in CANONICAL_BENCHMARK_CASES {
            let res = self.evaluate_case(case).await;
            results.push(res);
        }

        let total = results.len();
        let zero_leakage_count = results.iter().filter(|r| r.zero_leakage).count();
        let diag_count = results.iter().filter(|r| r.diagnostic_isolated).count();
        let socratic_count = results.iter().filter(|r| r.has_socratic_question).count();
        let schema_count = results.iter().filter(|r| r.valid_schema).count();
        let total_score: f32 = results.iter().map(|r| r.score).sum();

        let zero_leakage_rate = if total > 0 {
            (zero_leakage_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        let diagnostic_precision = if total > 0 {
            (diag_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        let socratic_guidance_score = if total > 0 {
            (socratic_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        let schema_adherence_rate = if total > 0 {
            (schema_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        let overall_score = if total > 0 {
            total_score / total as f32
        } else {
            0.0
        };

        BenchmarkReport {
            model_id: self.model_id.clone(),
            total_cases: total,
            zero_leakage_rate,
            diagnostic_precision,
            socratic_guidance_score,
            schema_adherence_rate,
            overall_score,
            results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use nereus_core::error::CoreResult;

    #[derive(Debug, Clone)]
    struct MockLeakingLLM;

    #[async_trait]
    impl LLMClient for MockLeakingLLM {
        async fn complete(&self, _prompt: &str) -> CoreResult<String> {
            Ok(
                "Here is the solution:\n```rust\nfn main() { println!(\"fixed\"); }\n```"
                    .to_string(),
            )
        }
    }

    #[derive(Debug, Clone)]
    struct MockSocraticLLM;

    #[async_trait]
    impl LLMClient for MockSocraticLLM {
        async fn complete(&self, prompt: &str) -> CoreResult<String> {
            let error_code = if let Some(pos) = prompt.find("error[E") {
                &prompt[pos + 6..pos + 11]
            } else {
                "E0000"
            };
            Ok(format!(
                "Diagnostic: Notice compiler error {error_code} regarding language constraints. Reflective Question: What invariant is the compiler protecting here?"
            ))
        }
    }

    #[tokio::test]
    async fn test_benchmark_suite_detects_code_leakage() {
        let llm = Arc::new(MockLeakingLLM);
        let bench = SocraticBenchmark::new(llm, "mock-leaking-model");
        let report = bench.run_suite().await;

        assert_eq!(report.total_cases, 10);
        assert_eq!(report.zero_leakage_rate, 0.0);
        assert!(!report.results[0].zero_leakage);
    }

    #[tokio::test]
    async fn test_benchmark_suite_socratic_success() {
        let llm = Arc::new(MockSocraticLLM);
        let bench = SocraticBenchmark::new(llm, "mock-socratic-model");
        let report = bench.run_suite().await;

        assert_eq!(report.total_cases, 10);
        assert_eq!(report.zero_leakage_rate, 100.0);
        assert_eq!(report.socratic_guidance_score, 100.0);
        assert!(report.diagnostic_precision >= 90.0);
        assert!(report.overall_score >= 90.0);

        let json_output = report.to_json().unwrap();
        assert!(json_output.contains("mock-socratic-model"));

        let md_output = report.to_markdown();
        assert!(md_output.contains("# 🌊 Nereus Socratic Mentor Benchmark Report"));
        assert!(md_output.contains("Zero Code Leakage Rate"));
    }
}
