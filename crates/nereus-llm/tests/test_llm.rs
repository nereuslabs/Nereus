//! Comprehensive unit tests for `nereus-llm`.
//! Tests cover Stub client, dynamic prompt templating, and structured retry logic.

use async_trait::async_trait;
use nereus_core::error::{CoreError, CoreResult};
use nereus_core::models::{Assessment, Verdict};
use nereus_core::traits::LLMClient;
use nereus_llm::{
    extract_json, parse_json, AnthropicClient, AnthropicModel, PromptRegistry, PromptTemplate,
    StructuredClient, StubLLMClient,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// 1. Stub LLM Client Tests
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct SimpleOutput {
    pub message: Option<String>,
}

#[tokio::test]
async fn test_stub_client_default_completion() {
    let client = StubLLMClient::new();
    let resp = client.complete("Evaluate this code").await.unwrap();
    assert!(
        resp.contains("Stub assessment"),
        "Default stub completion must mention stub assessment"
    );
    assert!(
        resp.contains("PASS"),
        "Default stub completion should report PASS"
    );
}

#[tokio::test]
async fn test_stub_client_custom_canned_completion() {
    let client = StubLLMClient::new().with_canned_completion("Custom deterministic response");
    let resp = client.complete("Any prompt").await.unwrap();
    assert_eq!(resp, "Custom deterministic response");
}

#[tokio::test]
async fn test_stub_client_default_structured_output() {
    let client = StubLLMClient::new();
    let parsed: SimpleOutput = client.structured_output("Generate output").await.unwrap();
    assert_eq!(parsed, SimpleOutput::default());
}

#[tokio::test]
async fn test_stub_client_canned_json_assessment() {
    let sample_json = r#"{
        "topic_id": "topic-ownership",
        "score": 92.5,
        "verdict": "pass",
        "feedback": "Flawless move semantics and lifetime scoping.",
        "weak_areas": []
    }"#;

    let client = StubLLMClient::new().with_canned_json(sample_json);
    let assessment: Assessment = client
        .structured_output("Assess student code")
        .await
        .unwrap();

    assert_eq!(assessment.topic_id, "topic-ownership");
    assert!((assessment.score - 92.5).abs() < f32::EPSILON);
    assert_eq!(assessment.verdict, Verdict::Pass);
    assert_eq!(
        assessment.feedback,
        "Flawless move semantics and lifetime scoping."
    );
    assert!(assessment.weak_areas.is_empty());
}

#[tokio::test]
async fn test_stub_client_markdown_fenced_json() {
    let fenced_json = "```json\n{\"topic_id\":\"fenced\",\"score\":100.0,\"verdict\":\"pass\",\"feedback\":\"Great\",\"weak_areas\":[]}\n```";
    let client = StubLLMClient::new().with_canned_json(fenced_json);
    let assessment: Assessment = client.structured_output("Prompt").await.unwrap();
    assert_eq!(assessment.topic_id, "fenced");
    assert_eq!(assessment.verdict, Verdict::Pass);
}

// ============================================================================
// 2. Prompt Templating & Registry Tests
// ============================================================================

#[test]
fn test_prompt_template_basic_interpolation() {
    let tpl = PromptTemplate::new("Hello, {{skill}}! Today we learn {{topic}}.");
    let rendered = tpl.render([("skill", "Rust"), ("topic", "Concurrency")]);
    assert_eq!(rendered, "Hello, Rust! Today we learn Concurrency.");
}

#[test]
fn test_prompt_template_all_five_variables() {
    let tpl = PromptTemplate::new(
        "Skill: {{skill}}\nTopic: {{topic}}\nWeaknesses: {{weak_areas}}\nSubmission: {{submission}}\nError: {{compiler_error}}"
    );

    let rendered = tpl.render([
        ("skill", "Rust"),
        ("topic", "Lifetimes"),
        ("weak_areas", "Borrow Checker, Sync"),
        ("submission", "fn foo<'a>(x: &'a str) -> &'a str { x }"),
        ("compiler_error", "none"),
    ]);

    assert!(rendered.contains("Skill: Rust"));
    assert!(rendered.contains("Topic: Lifetimes"));
    assert!(rendered.contains("Weaknesses: Borrow Checker, Sync"));
    assert!(rendered.contains("fn foo<'a>"));
    assert!(rendered.contains("Error: none"));
}

#[test]
fn test_prompt_template_render_map() {
    let tpl = PromptTemplate::new("Focus: {{skill}} -> {{topic}}");
    let mut map = HashMap::new();
    map.insert("skill".to_string(), "Systems Programming".to_string());
    map.insert("topic".to_string(), "Memory Layout".to_string());

    let rendered = tpl.render_map(&map);
    assert_eq!(rendered, "Focus: Systems Programming -> Memory Layout");
}

#[test]
fn test_prompt_template_repeated_variable() {
    let tpl = PromptTemplate::new("Master {{skill}}! Teach {{skill}}! Love {{skill}}!");
    let rendered = tpl.render([("skill", "Rust")]);
    assert_eq!(rendered, "Master Rust! Teach Rust! Love Rust!");
}

#[test]
fn test_prompt_template_variable_discovery() {
    let tpl = PromptTemplate::new(
        "Skill: {{skill}}, Topic: {{topic}}, Weaknesses: {{weak_areas}}, Error: {{compiler_error}}",
    );
    let vars = tpl.variables();
    assert_eq!(vars, vec!["skill", "topic", "weak_areas", "compiler_error"]);
}

#[test]
fn test_prompt_template_unmatched_placeholder_remains() {
    let tpl = PromptTemplate::new("Skill: {{skill}}, Unset: {{missing}}");
    let rendered = tpl.render([("skill", "Rust")]);
    assert_eq!(rendered, "Skill: Rust, Unset: {{missing}}");
}

#[test]
fn test_prompt_registry_defaults_exist() {
    let registry = PromptRegistry::new();
    assert!(registry.contains("diagnostic"));
    assert!(registry.contains("coach"));
    assert!(registry.contains("tutor"));
    assert!(registry.contains("examiner"));
    assert!(registry.contains("diagnostic_system"));
    assert!(registry.contains("coach_system"));
    assert!(registry.contains("tutor_system"));
    assert!(registry.contains("examiner_system"));
}

#[test]
fn test_prompt_registry_render_diagnostic() {
    let registry = PromptRegistry::new();
    let rendered = registry.render_diagnostic("Rust").unwrap();
    assert!(rendered.contains("skill 'Rust'"));
    assert!(rendered.contains("memory safety, concurrency, and idioms"));
}

#[test]
fn test_prompt_registry_render_coach() {
    let registry = PromptRegistry::new();
    let rendered = registry
        .render_coach("Rust", "Lifetimes, Tokio Channels")
        .unwrap();
    assert!(rendered.contains("skill 'Rust'"));
    assert!(rendered.contains("weak areas are: Lifetimes, Tokio Channels"));
}

#[test]
fn test_prompt_registry_render_tutor() {
    let registry = PromptRegistry::new();
    let rendered = registry
        .render_tutor("Rust", "Arc & Mutex Patterns")
        .unwrap();
    assert!(rendered.contains("topic 'Arc & Mutex Patterns'"));
    assert!(rendered.contains("skill 'Rust'"));
}

#[test]
fn test_prompt_registry_render_examiner() {
    let registry = PromptRegistry::new();
    let rendered = registry
        .render_examiner(
            "Borrow Checker",
            "let mut x = 5; let y = &x; x = 10;",
            "cannot assign to `x` because it is borrowed",
        )
        .unwrap();

    assert!(rendered.contains("topic 'Borrow Checker'"));
    assert!(rendered.contains("let mut x = 5; let y = &x; x = 10;"));
    assert!(rendered.contains("cannot assign to `x` because it is borrowed"));
    assert!(rendered.contains("Socratic feedback"));
}

#[test]
fn test_prompt_registry_custom_registration() {
    let mut registry = PromptRegistry::new();
    registry.register(
        "socratic_hint",
        PromptTemplate::new(
            "What does the compiler mean by '{{error_code}}' in topic '{{topic}}'?",
        ),
    );

    let rendered = registry
        .render(
            "socratic_hint",
            [("error_code", "E0382"), ("topic", "Ownership")],
        )
        .unwrap();

    assert_eq!(
        rendered,
        "What does the compiler mean by 'E0382' in topic 'Ownership'?"
    );
}

#[test]
fn test_prompt_registry_missing_template_error() {
    let registry = PromptRegistry::new();
    let err = registry.render("nonexistent_agent", []).unwrap_err();
    match err {
        CoreError::Internal(msg) => {
            assert!(msg.contains("Prompt template 'nonexistent_agent' not found"));
        }
        other => panic!("Expected CoreError::Internal, got {other:?}"),
    }
}

// ============================================================================
// 3. Structured Inference Client with Retry Logic Tests
// ============================================================================

/// Mock LLM Client that returns programmed responses in sequence and records received prompts.
#[derive(Debug, Clone)]
struct MockSequenceLLMClient {
    responses: Arc<Mutex<Vec<String>>>,
    received_prompts: Arc<Mutex<Vec<String>>>,
    call_count: Arc<AtomicUsize>,
}

impl MockSequenceLLMClient {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            received_prompts: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    async fn received_prompts(&self) -> Vec<String> {
        self.received_prompts.lock().await.clone()
    }

    fn calls(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LLMClient for MockSequenceLLMClient {
    async fn complete(&self, prompt: &str) -> CoreResult<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.received_prompts.lock().await.push(prompt.to_string());

        let mut resps = self.responses.lock().await;
        if resps.is_empty() {
            Err(CoreError::LLMError(
                "No more programmed responses".to_string(),
            ))
        } else {
            Ok(resps.remove(0))
        }
    }
}

#[tokio::test]
async fn test_structured_client_first_attempt_success() {
    let valid_json = r#"{
        "topic_id": "topic-1",
        "score": 95.0,
        "verdict": "pass",
        "feedback": "Perfect solution.",
        "weak_areas": []
    }"#;

    let mock = MockSequenceLLMClient::new(vec![valid_json.to_string()]);
    let client = StructuredClient::new(mock.clone());

    let assessment: Assessment = client
        .structured_output("Assess topic 1")
        .await
        .expect("Should parse on first attempt");

    assert_eq!(mock.calls(), 1, "Only 1 call should be made on clean JSON");
    assert_eq!(assessment.topic_id, "topic-1");
    assert_eq!(assessment.verdict, Verdict::Pass);
}

#[tokio::test]
async fn test_structured_client_extracts_markdown_code_fence() {
    let fenced_response = "Here is the evaluation:\n```json\n{\n  \"topic_id\": \"topic-fenced\",\n  \"score\": 88.0,\n  \"verdict\": \"pass\",\n  \"feedback\": \"Good job\",\n  \"weak_areas\": []\n}\n```\nHope this helps!";

    let mock = MockSequenceLLMClient::new(vec![fenced_response.to_string()]);
    let client = StructuredClient::new(mock.clone());

    let assessment: Assessment = client
        .structured_output("Assess topic fenced")
        .await
        .expect("Should extract and parse markdown-fenced JSON");

    assert_eq!(mock.calls(), 1);
    assert_eq!(assessment.topic_id, "topic-fenced");
    assert_eq!(assessment.score, 88.0);
}

#[tokio::test]
async fn test_structured_client_retry_once_success() {
    let invalid_json = "I cannot provide JSON right now, here is plain text.";
    let valid_json = r#"{
        "topic_id": "topic-2",
        "score": 75.0,
        "verdict": "pass",
        "feedback": "Passed after clarification.",
        "weak_areas": []
    }"#;

    let mock = MockSequenceLLMClient::new(vec![invalid_json.to_string(), valid_json.to_string()]);
    let client = StructuredClient::new(mock.clone());

    let assessment: Assessment = client
        .structured_output("Evaluate student")
        .await
        .expect("Should succeed on retry 1");

    assert_eq!(mock.calls(), 2, "Expected exactly 2 attempts");
    assert_eq!(assessment.topic_id, "topic-2");

    let prompts = mock.received_prompts().await;
    assert_eq!(prompts.len(), 2);
    assert_eq!(prompts[0], "Evaluate student");

    // Verify error feedback prompt content
    assert!(
        prompts[1].contains("Previous output was invalid JSON:"),
        "Retry prompt must contain 'Previous output was invalid JSON:' error feedback"
    );
    assert!(
        prompts[1].contains("Evaluate student"),
        "Retry prompt should retain original prompt context"
    );
}

#[tokio::test]
async fn test_structured_client_retry_twice_success() {
    let invalid_json_1 = "{ bad json syntax ... ";
    let invalid_json_2 = "{ \"score\": \"not a float\" }";
    let valid_json = r#"{
        "topic_id": "topic-3",
        "score": 85.0,
        "verdict": "pass",
        "feedback": "Passed on final allowed retry.",
        "weak_areas": []
    }"#;

    let mock = MockSequenceLLMClient::new(vec![
        invalid_json_1.to_string(),
        invalid_json_2.to_string(),
        valid_json.to_string(),
    ]);
    let client = StructuredClient::new(mock.clone());

    let assessment: Assessment = client
        .structured_output("Evaluate topic 3")
        .await
        .expect("Should succeed on 2nd retry (3rd total attempt)");

    assert_eq!(
        mock.calls(),
        3,
        "Expected 3 attempts (1 initial + 2 retries)"
    );
    assert_eq!(assessment.topic_id, "topic-3");

    let prompts = mock.received_prompts().await;
    assert_eq!(prompts.len(), 3);
    assert!(prompts[1].contains("Previous output was invalid JSON:"));
    assert!(prompts[2].contains("Previous output was invalid JSON:"));
}

#[tokio::test]
async fn test_structured_client_exhausted_retries_fails() {
    let invalid_1 = "invalid 1";
    let invalid_2 = "invalid 2";
    let invalid_3 = "invalid 3";

    let mock = MockSequenceLLMClient::new(vec![
        invalid_1.to_string(),
        invalid_2.to_string(),
        invalid_3.to_string(),
    ]);
    // Default max_retries is 2 -> 3 total attempts
    let client = StructuredClient::new(mock.clone());

    let result: CoreResult<Assessment> = client.structured_output("Assess failure").await;

    assert!(result.is_err(), "Must return error when retries exhausted");
    assert_eq!(mock.calls(), 3, "Should have attempted exactly 3 times");

    match result.unwrap_err() {
        CoreError::Serialization(msg) => {
            assert!(
                msg.contains("Previous output was invalid JSON:"),
                "Final error must state previous output was invalid JSON"
            );
        }
        other => panic!("Expected CoreError::Serialization, got {other:?}"),
    }
}

#[tokio::test]
async fn test_structured_client_custom_max_retries() {
    let invalid_1 = "invalid 1";
    let invalid_2 = "invalid 2";

    let mock = MockSequenceLLMClient::new(vec![invalid_1.to_string(), invalid_2.to_string()]);
    // Configure only 1 retry -> 2 total attempts
    let client = StructuredClient::with_max_retries(mock.clone(), 1);

    let result: CoreResult<Assessment> = client.structured_output("Assess").await;
    assert!(result.is_err());
    assert_eq!(mock.calls(), 2, "Should stop after 1 retry (2 calls total)");
}

#[tokio::test]
async fn test_structured_client_complete_passthrough() {
    let mock = MockSequenceLLMClient::new(vec!["Pass-through text response".to_string()]);
    let client = StructuredClient::new(mock.clone());

    let resp = client.complete("Hello").await.unwrap();
    assert_eq!(resp, "Pass-through text response");
    assert_eq!(mock.calls(), 1);
}

// ============================================================================
// 4. Anthropic Client Unit Tests
// ============================================================================

#[test]
fn test_anthropic_model_identifiers() {
    assert_eq!(
        AnthropicModel::Claude35Sonnet.as_str(),
        "claude-3-5-sonnet-20241022"
    );
    assert_eq!(
        AnthropicModel::Claude3Opus.as_str(),
        "claude-3-opus-20240229"
    );
    assert_eq!(
        AnthropicModel::Claude3Haiku.as_str(),
        "claude-3-haiku-20240307"
    );

    let custom = AnthropicModel::from("claude-next-gen");
    assert_eq!(custom.as_str(), "claude-next-gen");

    assert_eq!(
        AnthropicModel::from("claude-3-5-sonnet"),
        AnthropicModel::Claude35Sonnet
    );
    assert_eq!(
        AnthropicModel::from("claude-3-opus"),
        AnthropicModel::Claude3Opus
    );
}

#[test]
fn test_anthropic_client_builder() {
    let client = AnthropicClient::new("test-key")
        .set_model(AnthropicModel::Claude3Opus)
        .with_max_tokens(2048)
        .with_base_url("http://localhost:8080/v1/messages")
        .with_system("You are a tutor.");

    assert_eq!(client.model(), "claude-3-opus-20240229");
    assert_eq!(client.base_url(), "http://localhost:8080/v1/messages");

    let client_opus = AnthropicClient::claude_3_opus("test-key");
    assert_eq!(client_opus.model(), "claude-3-opus-20240229");

    let client_with_model = AnthropicClient::with_model("test-key", AnthropicModel::Claude35Sonnet);
    assert_eq!(client_with_model.model(), "claude-3-5-sonnet-20241022");
}

#[tokio::test]
async fn test_anthropic_client_network_error_mapping() {
    // Point to non-routable address to verify error mapping to CoreError::LLMError
    let client = AnthropicClient::new("fake-key").with_base_url("http://127.0.0.1:1/nonexistent");

    let result = client.complete("Test prompt").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::LLMError(msg) => {
            assert!(
                msg.contains("Anthropic network error") || msg.contains("error"),
                "Expected network error message, got {msg}"
            );
        }
        other => panic!("Expected CoreError::LLMError, got {other:?}"),
    }
}

// ============================================================================
// 5. JSON Extraction Helper Tests
// ============================================================================

#[test]
fn test_extract_json_utilities() {
    assert_eq!(extract_json(r#"{"a": 1}"#), r#"{"a": 1}"#);
    assert_eq!(extract_json(r#"[1, 2, 3]"#), r#"[1, 2, 3]"#);

    let fenced_json = "```json\n{\"score\": 90}\n```";
    assert_eq!(extract_json(fenced_json), "{\"score\": 90}");

    let fenced_generic = "```\n[10, 20]\n```";
    assert_eq!(extract_json(fenced_generic), "[10, 20]");

    let commentary = "Note: here is the result: {\"success\": true} done.";
    assert_eq!(extract_json(commentary), "{\"success\": true}");

    // Direct parser helper
    #[derive(Deserialize, PartialEq, Debug)]
    struct Score {
        score: u32,
    }
    let parsed: Score = parse_json(fenced_json).unwrap();
    assert_eq!(parsed, Score { score: 90 });
}
