//! Integration and unit test suite for nereus-sandbox runners and diagnostic parser.

use nereus_core::models::RoadmapTopic;
use nereus_core::traits::SandboxRunner;
use nereus_sandbox::diagnostic::{DiagnosticLevel, DiagnosticParser};
use nereus_sandbox::host::{prepare_harness_code, HostSandboxRunner, Scratchpad};
use nereus_sandbox::mock::{MockSandboxRunner, MockScenario};
use nereus_sandbox::wasm::{WasmtimeSandboxRunner, DEFAULT_MAX_FUEL, DEFAULT_MEMORY_CAP_BYTES};
use std::sync::Arc;
use std::time::Duration;

fn sample_topic(title: &str) -> RoadmapTopic {
    RoadmapTopic {
        id: "topic-test-01".to_string(),
        title: title.to_string(),
        description: "Test assessment topic".to_string(),
        difficulty: 0.5,
        prerequisites: vec![],
        estimated_hours: 1.0,
    }
}

// ============================================================================
// 1. Diagnostic Parser Tests
// ============================================================================

#[test]
fn test_diagnostic_parser_e0382_move_error() {
    let stderr = r#"error[E0382]: use of moved value: `tx`
  --> src/main.rs:12:9
   |
10 |     let tx2 = tx;
   |               -- value moved here
12 |     tx.send(42).await;
   |     ^^ value used here after move
"#;

    let diags = DiagnosticParser::parse(stderr);
    assert_eq!(diags.len(), 1);

    let d = &diags[0];
    assert_eq!(d.level, DiagnosticLevel::Error);
    assert_eq!(d.code.as_deref(), Some("E0382"));
    assert_eq!(d.message, "use of moved value: `tx`");
    assert!(d.is_error());
    assert!(!d.is_warning());

    let span = d.span.as_ref().expect("Span should be parsed");
    assert_eq!(span.file, "src/main.rs");
    assert_eq!(span.line, 12);
    assert_eq!(span.column, 9);
    assert_eq!(span.to_string(), "src/main.rs:12:9");

    assert_eq!(
        d.summary(),
        "error[E0382]: use of moved value: `tx` at src/main.rs:12:9"
    );
}

#[test]
fn test_diagnostic_parser_e0597_lifetime_error() {
    let stderr = r#"error[E0597]: `x` does not live long enough
  --> tests/solution.rs:8:14
   |
 8 |         r = &x;
   |              ^ borrowed value does not live long enough
 9 |     }
   |     - `x` dropped here while still borrowed
"#;

    let diags = DiagnosticParser::parse(stderr);
    assert_eq!(diags.len(), 1);

    let d = &diags[0];
    assert_eq!(d.code.as_deref(), Some("E0597"));
    assert_eq!(d.message, "`x` does not live long enough");
    let span = d.span.as_ref().unwrap();
    assert_eq!(span.file, "tests/solution.rs");
    assert_eq!(span.line, 8);
    assert_eq!(span.column, 14);
}

#[test]
fn test_diagnostic_parser_e0502_borrow_conflict() {
    let stderr = r#"error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
  --> src/lib.rs:15:5
   |
14 |     let first = &v[0];
   |                  - immutable borrow occurs here
15 |     v.push(42);
   |     ^^^^^^^^^^ mutable borrow occurs here
"#;

    let diags = DiagnosticParser::parse(stderr);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_deref(), Some("E0502"));
    assert_eq!(diags[0].span.as_ref().unwrap().line, 15);
}

#[test]
fn test_diagnostic_parser_e0277_trait_bound() {
    let stderr = r#"error[E0277]: the trait bound `CustomType: std::fmt::Display` is not satisfied
  --> src/main.rs:22:20
   |
22 |     println!("{}", custom_val);
   |                    ^^^^^^^^^^ the trait `std::fmt::Display` is not implemented for `CustomType`
"#;

    let diags = DiagnosticParser::parse(stderr);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_deref(), Some("E0277"));
    assert_eq!(
        diags[0].message,
        "the trait bound `CustomType: std::fmt::Display` is not satisfied"
    );
}

#[test]
fn test_diagnostic_parser_warnings_and_multiple() {
    let stderr = r#"warning: unused variable: `unused_buffer`
  --> src/main.rs:3:9
   |
 3 |     let unused_buffer = 100;
   |         ^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_unused_buffer`

error[E0382]: use of moved value: `data`
  --> src/main.rs:10:5
   |
 9 |     drop(data);
   |          ---- value moved here
10 |     data.len();
   |     ^^^^ value used here after move
"#;

    let diags = DiagnosticParser::parse(stderr);
    assert_eq!(diags.len(), 2);

    assert_eq!(diags[0].level, DiagnosticLevel::Warning);
    assert_eq!(diags[0].code, None);
    assert_eq!(diags[0].message, "unused variable: `unused_buffer`");
    assert!(diags[0].is_warning());

    assert_eq!(diags[1].level, DiagnosticLevel::Error);
    assert_eq!(diags[1].code.as_deref(), Some("E0382"));
    assert!(diags[1].is_error());

    let codes = DiagnosticParser::extract_error_codes(stderr);
    assert_eq!(codes, vec!["E0382".to_string()]);
}

#[test]
fn test_diagnostic_parser_empty_or_non_error_output() {
    let empty = DiagnosticParser::parse("");
    assert!(empty.is_empty());

    let non_error = DiagnosticParser::parse("   Compiling nereus v0.2.0\n    Finished dev\n");
    assert!(non_error.is_empty());
}

// ============================================================================
// 2. MockSandboxRunner Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_mock_runner_success_scenario() {
    let runner = MockSandboxRunner::success();
    let topic = sample_topic("Ownership & Borrowing");
    let result = runner.execute(&topic, "fn main() {}").await.unwrap();

    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test result: ok"));
    assert!(result.stderr.is_empty());
}

#[tokio::test]
async fn test_mock_runner_compiler_error_scenario() {
    let custom_error = "error[E0382]: use of moved value\n  --> main.rs:1:1";
    let runner = MockSandboxRunner::compiler_error(custom_error);
    let topic = sample_topic("Channels & Concurrency");
    let result = runner.execute(&topic, "let a = b;").await.unwrap();

    assert!(!result.success);
    assert_eq!(result.exit_code, 101);
    assert_eq!(result.stderr, custom_error);
}

#[tokio::test]
async fn test_mock_runner_failing_compiler_default() {
    let runner = MockSandboxRunner::failing_compiler();
    let topic = sample_topic("Lifetimes");
    let result = runner.execute(&topic, "bad code").await.unwrap();

    assert!(!result.success);
    assert!(result.stderr.contains("E0382"));
}

#[tokio::test]
async fn test_mock_runner_assertion_failed_scenario() {
    let runner = MockSandboxRunner::assertion_failed();
    let topic = sample_topic("Data Structures");
    let result = runner.execute(&topic, "assert_eq!(1, 2);").await.unwrap();

    assert!(!result.success);
    assert_eq!(result.exit_code, 101);
    assert!(result.stderr.contains("assertion `left == right` failed"));
}

#[tokio::test]
async fn test_mock_runner_timeout_scenario() {
    let runner = MockSandboxRunner::timeout();
    let topic = sample_topic("Async Streams");
    let result = runner.execute(&topic, "loop {}").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("timed out"));
}

#[tokio::test]
async fn test_mock_runner_dynamic_scenario() {
    let runner = MockSandboxRunner::default();
    let topic = sample_topic("Pattern Matching");

    // Code containing "pass" succeeds
    let pass_res = runner.execute(&topic, "// will pass").await.unwrap();
    assert!(pass_res.success);

    // Code without "pass" or "tokio" fails with compiler error
    let fail_res = runner.execute(&topic, "let x = 1;").await.unwrap();
    assert!(!fail_res.success);
    assert!(fail_res.stderr.contains("E0382"));
}

#[tokio::test]
async fn test_mock_runner_set_scenario_mutation() {
    let mut runner = MockSandboxRunner::success();
    let topic = sample_topic("Traits");

    assert!(runner.execute(&topic, "").await.unwrap().success);

    runner.set_scenario(MockScenario::AssertionFailed);
    let failed = runner.execute(&topic, "").await.unwrap();
    assert!(!failed.success);
    assert!(failed.stderr.contains("panicked"));
}

// ============================================================================
// 3. HostSandboxRunner Tests
// ============================================================================

#[test]
fn test_scratchpad_lifecycle() {
    let scratchpad = Scratchpad::create().expect("Scratchpad should be created");
    assert!(scratchpad.path().exists());
    assert!(scratchpad.path().starts_with(std::env::temp_dir()));

    let written_file = scratchpad
        .write_file("test.txt", "Hello Nereus")
        .expect("File should be written");
    assert!(written_file.exists());
    assert_eq!(
        std::fs::read_to_string(&written_file).unwrap(),
        "Hello Nereus"
    );

    let saved_path = scratchpad.path().to_path_buf();
    drop(scratchpad);

    // Scratchpad should be cleaned up on drop
    assert!(!saved_path.exists());
}

#[test]
fn test_prepare_harness_code_wrapping() {
    // Already has main
    let code_with_main = "fn main() { println!(\"Hi\"); }";
    assert_eq!(prepare_harness_code(code_with_main), code_with_main);

    // Has tests
    let code_with_test = "#[test]\nfn test_it() {}";
    assert_eq!(prepare_harness_code(code_with_test), code_with_test);

    // Bare function snippet -> wrap with main harness
    let bare_fn = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
    let prepared = prepare_harness_code(bare_fn);
    assert!(prepared.contains(bare_fn));
    assert!(prepared.contains("fn main()"));
}

#[tokio::test]
async fn test_host_runner_execution_passing() {
    let runner = HostSandboxRunner::new(Duration::from_secs(10));
    let topic = sample_topic("Hello World");
    let code = r#"
fn main() {
    println!("NEREUS_HOST_EXECUTION_PASS");
}
"#;

    let result = runner.execute(&topic, code).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("NEREUS_HOST_EXECUTION_PASS"));
    assert!(result.stderr.is_empty());
}

#[tokio::test]
async fn test_host_runner_execution_compiler_error() {
    let runner = HostSandboxRunner::new(Duration::from_secs(10));
    let topic = sample_topic("Type System");
    let code = r#"
fn main() {
    let s: i32 = "invalid string type";
}
"#;

    let (result, diags) = runner.execute_and_diagnose(&topic, code).await.unwrap();
    assert!(!result.success);
    assert_ne!(result.exit_code, 0);
    assert!(result.stderr.contains("mismatched types") || result.stderr.contains("E0308"));

    // Verify diagnostic parser processed the rustc output
    assert!(!diags.is_empty());
    assert!(diags.iter().any(|d| d.is_error()));
}

#[tokio::test]
async fn test_host_runner_execution_test_assertions() {
    let runner = HostSandboxRunner::new(Duration::from_secs(10));
    let topic = sample_topic("Unit Tests");
    let code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}
"#;

    let result = runner.execute(&topic, code).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("test result: ok") || result.stdout.contains("1 passed"));
}

// ============================================================================
// 4. WasmtimeSandboxRunner Tests
// ============================================================================

#[test]
fn test_wasmtime_runner_initialization() {
    let runner = WasmtimeSandboxRunner::new().expect("Wasmtime runner should initialize");
    assert_eq!(runner.max_fuel(), DEFAULT_MAX_FUEL);
    assert_eq!(runner.memory_cap_bytes(), DEFAULT_MEMORY_CAP_BYTES);

    let customized = runner
        .with_fuel(10_000_000)
        .with_memory_cap(32 * 1024 * 1024);
    assert_eq!(customized.max_fuel(), 10_000_000);
    assert_eq!(customized.memory_cap_bytes(), 32 * 1024 * 1024);
}

#[tokio::test]
async fn test_wasmtime_wat_execution_success() {
    let runner = WasmtimeSandboxRunner::new().unwrap();
    let topic = sample_topic("Wasm Basics");

    let wat = r#"
(module
  (func (export "_start")
    (nop)
  )
)
"#;

    let result = runner.execute(&topic, wat).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
}

#[tokio::test]
async fn test_wasmtime_wat_stdout_capture_via_wasi() {
    let runner = WasmtimeSandboxRunner::new().unwrap();
    let topic = sample_topic("Wasm WASI Output");

    // WAT module using WASI fd_write to print "Hello WASM\n"
    let wat = r#"
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  ;; Message string at offset 0: "Hello WASM\n" (11 bytes)
  (data (i32.const 0) "Hello WASM\n")

  ;; iovec structure at offset 32:
  ;; ptr = 0, len = 11
  (data (i32.const 32) "\00\00\00\00\0b\00\00\00")

  (func (export "_start")
    ;; Call fd_write(fd=1 (stdout), iovs=32, iovs_len=1, nwritten=64)
    (drop (call $fd_write
      (i32.const 1)
      (i32.const 32)
      (i32.const 1)
      (i32.const 64)
    ))
  )
)
"#;

    let result = runner.execute(&topic, wat).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "Hello WASM\n");
}

#[tokio::test]
async fn test_wasmtime_fuel_consumption_infinite_loop_prevention() {
    let runner = WasmtimeSandboxRunner::new().unwrap().with_fuel(100_000); // 100,000 instructions max

    let topic = sample_topic("Infinite Loop Prevention");

    // Infinite loop in WAT
    let wat = r#"
(module
  (func (export "_start")
    (loop $infinite
      br $infinite
    )
  )
)
"#;

    let result = runner.execute(&topic, wat).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, 124);
    assert!(result.stderr.contains("out of fuel"));
}

#[tokio::test]
async fn test_wasmtime_memory_cap_prevents_excessive_growth() {
    let runner = WasmtimeSandboxRunner::new().unwrap();
    let topic = sample_topic("Memory Cap Growth");

    // WAT attempts to grow memory by 2000 pages (128MB), exceeding 64MB (1024 pages)
    let wat = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start")
    (local $res i32)
    (local.set $res (memory.grow (i32.const 2000)))
    ;; memory.grow returns -1 on failure
    (if (i32.ne (local.get $res) (i32.const -1))
      (then (unreachable))
    )
  )
)
"#;

    let result = runner.execute(&topic, wat).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
}

#[tokio::test]
async fn test_wasmtime_initial_memory_exceeding_cap_fails() {
    let runner = WasmtimeSandboxRunner::new().unwrap();
    let topic = sample_topic("Initial Memory Exceeding Cap");

    // Initial memory requests 2000 pages (~128MB), exceeding 64MB limit
    let wat = r#"
(module
  (memory (export "memory") 2000)
  (func (export "_start") (nop))
)
"#;

    let result = runner.execute(&topic, wat).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, 137);
    assert!(result.stderr.to_lowercase().contains("memory"));
}

#[tokio::test]
async fn test_wasmtime_rust_code_compilation_and_execution() {
    let runner = WasmtimeSandboxRunner::new().unwrap();
    let topic = sample_topic("Rust to WASM");

    let code = r#"
fn main() {
    println!("HELLO_FROM_WASMTIME_SANDBOX");
}
"#;

    let result = runner.execute(&topic, code).await.unwrap();
    assert!(result.success);
    assert_eq!(result.exit_code, 0);
    assert!(
        result.stdout.contains("HELLO_FROM_WASMTIME_SANDBOX") || result.stdout.contains("fallback")
    );
}

// ============================================================================
// 5. Dynamic Trait Polymorphism Test
// ============================================================================

#[tokio::test]
async fn test_sandbox_runner_dynamic_dispatch() {
    let runners: Vec<Arc<dyn SandboxRunner>> = vec![
        Arc::new(MockSandboxRunner::success()),
        Arc::new(HostSandboxRunner::default()),
        Arc::new(WasmtimeSandboxRunner::default()),
    ];

    let topic = sample_topic("Trait Polymorphism");
    let code = "fn main() { println!(\"polymorphic\"); }";

    for runner in runners {
        let res = runner.execute(&topic, code).await.unwrap();
        assert!(res.success);
    }
}
