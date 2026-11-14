//! Dynamic prompt templates, variable interpolation, and system instructions for Nereus agents.

use nereus_core::error::{CoreError, CoreResult};
use std::collections::HashMap;

/// System prompt instruction for the Diagnostic Agent.
pub const DIAGNOSTIC_SYSTEM_PROMPT: &str = r#"You are the Nereus Diagnostic Agent. Your role is to formulate targeted, diagnostic micro-questions to evaluate the learner's baseline. Focus on memory safety, concurrency, error handling, and language-specific idioms."#;

/// System prompt instruction for the Coach Agent.
pub const COACH_SYSTEM_PROMPT: &str = r#"You are the Nereus Coach Agent. Your role is to assemble an ordered Roadmap of RoadmapTopics that targets the student's weaknesses first. Keep milestones bite-sized (1-4 hours each)."#;

/// System prompt instruction for the Tutor Agent.
pub const TUTOR_SYSTEM_PROMPT: &str = r#"You are the Nereus Tutor Agent. Provide clear, concise conceptual material followed by an actionable coding task. On revisions, highlight conceptual gaps without providing the direct answer."#;

/// System prompt instruction for the Examiner Agent.
pub const EXAMINER_SYSTEM_PROMPT: &str = r#"You are the Nereus Examiner Agent. Adhere strictly to Socratic feedback principles:
1. Identify failing assertions or compiler error codes.
2. Formulate a guided question leading the student to the root cause.
3. Never give away the exact code or syntax."#;

/// Default diagnostic prompt template evaluating baseline skills.
pub const DIAGNOSTIC_TEMPLATE: &str = r#"Evaluate the learner's baseline knowledge for the skill '{{skill}}'.
Formulate targeted diagnostic questions focusing on memory safety, concurrency, and idioms.
Identify prerequisite gaps before advancing to the roadmap stage."#;

/// Default coach prompt template synthesizing roadmaps targeting weak areas.
pub const COACH_TEMPLATE: &str = r#"As the Nereus Coach Agent, compile an adaptive learning roadmap for skill '{{skill}}'.
The learner's identified weak areas are: {{weak_areas}}.
Assemble an ordered sequence of bite-sized topics prioritizing these weak areas first."#;

/// Default tutor prompt template explaining concepts and presenting coding exercises.
pub const TUTOR_TEMPLATE: &str = r#"As the Nereus Tutor Agent, teach the topic '{{topic}}' for skill '{{skill}}'.
Provide concise conceptual foundations and define an actionable, hands-on coding task for the learner."#;

/// Default examiner prompt template evaluating student code and compiler diagnostics.
pub const EXAMINER_TEMPLATE: &str = r#"As the Nereus Examiner Agent, assess the learner's submission for topic '{{topic}}'.

Student Submission:
{{submission}}

Compiler / Sandbox Diagnostics:
{{compiler_error}}

Adhere strictly to Socratic feedback: guide the student to the root cause without providing direct code or syntax."#;

/// A reusable prompt template supporting variable interpolation using `{{variable}}` syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    raw: String,
}

impl PromptTemplate {
    /// Creates a new template from raw text containing `{{placeholder}}` markers.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            raw: template.into(),
        }
    }

    /// Returns the underlying raw template string.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Interpolates variables provided as an iterator of key-value pairs.
    pub fn render<'a, I>(&self, vars: I) -> String
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut rendered = self.raw.clone();
        for (key, val) in vars {
            let placeholder = format!("{{{{{key}}}}}");
            rendered = rendered.replace(&placeholder, val);
        }
        rendered
    }

    /// Interpolates variables provided in a [`HashMap`].
    pub fn render_map(&self, vars: &HashMap<String, String>) -> String {
        let mut rendered = self.raw.clone();
        for (key, val) in vars {
            let placeholder = format!("{{{{{key}}}}}");
            rendered = rendered.replace(&placeholder, val);
        }
        rendered
    }

    /// Scans the template and returns a list of placeholder variable names.
    pub fn variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        let mut cursor = 0;
        while let Some(start) = self.raw[cursor..].find("{{") {
            let actual_start = cursor + start + 2;
            if let Some(end) = self.raw[actual_start..].find("}}") {
                let var_name = self.raw[actual_start..actual_start + end]
                    .trim()
                    .to_string();
                if !vars.contains(&var_name) {
                    vars.push(var_name);
                }
                cursor = actual_start + end + 2;
            } else {
                break;
            }
        }
        vars
    }
}

impl From<&str> for PromptTemplate {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for PromptTemplate {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Central registry managing dynamic prompt templates for all Nereus agents.
#[derive(Debug, Clone)]
pub struct PromptRegistry {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptRegistry {
    /// Initializes a registry pre-loaded with default templates for Diagnostic, Coach, Tutor, and Examiner.
    pub fn new() -> Self {
        let mut registry = Self {
            templates: HashMap::new(),
        };

        // Core prompt templates with interpolation placeholders
        registry.register("diagnostic", PromptTemplate::new(DIAGNOSTIC_TEMPLATE));
        registry.register("coach", PromptTemplate::new(COACH_TEMPLATE));
        registry.register("tutor", PromptTemplate::new(TUTOR_TEMPLATE));
        registry.register("examiner", PromptTemplate::new(EXAMINER_TEMPLATE));

        // System prompts
        registry.register(
            "diagnostic_system",
            PromptTemplate::new(DIAGNOSTIC_SYSTEM_PROMPT),
        );
        registry.register("coach_system", PromptTemplate::new(COACH_SYSTEM_PROMPT));
        registry.register("tutor_system", PromptTemplate::new(TUTOR_SYSTEM_PROMPT));
        registry.register(
            "examiner_system",
            PromptTemplate::new(EXAMINER_SYSTEM_PROMPT),
        );

        registry
    }

    /// Registers a new template or replaces an existing one by name.
    pub fn register(&mut self, name: impl Into<String>, template: impl Into<PromptTemplate>) {
        self.templates.insert(name.into(), template.into());
    }

    /// Retrieves a reference to a registered template.
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Returns `true` if a template is registered under the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Renders a template by name with the given key-value pairs.
    pub fn render<'a, I>(&self, name: &str, vars: I) -> CoreResult<String>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let template = self.templates.get(name).ok_or_else(|| {
            CoreError::Internal(format!("Prompt template '{name}' not found in registry"))
        })?;
        Ok(template.render(vars))
    }

    /// Convenience renderer for Diagnostic agent template with `{{skill}}`.
    pub fn render_diagnostic(&self, skill: &str) -> CoreResult<String> {
        self.render("diagnostic", [("skill", skill)])
    }

    /// Convenience renderer for Coach agent template with `{{skill}}` and `{{weak_areas}}`.
    pub fn render_coach(&self, skill: &str, weak_areas: &str) -> CoreResult<String> {
        self.render("coach", [("skill", skill), ("weak_areas", weak_areas)])
    }

    /// Convenience renderer for Tutor agent template with `{{skill}}` and `{{topic}}`.
    pub fn render_tutor(&self, skill: &str, topic: &str) -> CoreResult<String> {
        self.render("tutor", [("skill", skill), ("topic", topic)])
    }

    /// Convenience renderer for Examiner agent template with `{{topic}}`, `{{submission}}`, and `{{compiler_error}}`.
    pub fn render_examiner(
        &self,
        topic: &str,
        submission: &str,
        compiler_error: &str,
    ) -> CoreResult<String> {
        self.render(
            "examiner",
            [
                ("topic", topic),
                ("submission", submission),
                ("compiler_error", compiler_error),
            ],
        )
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
