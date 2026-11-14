//! Compiler diagnostic parsing for rustc error messages.
//!
//! Extracts rustc error codes (e.g. E0382, E0597, E0502, E0277), source file spans,
//! and human-readable error messages from standard compiler output.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Severity level of a compiler diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

impl std::fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Note => write!(f, "note"),
            Self::Help => write!(f, "help"),
        }
    }
}

/// Source code location span where a diagnostic occurred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSpan {
    /// File path where diagnostic was triggered
    pub file: String,
    /// 1-based source line number
    pub line: usize,
    /// 1-based source column number
    pub column: usize,
}

impl std::fmt::Display for DiagnosticSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Structured representation of a rustc compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    /// Severity level (error, warning, note, help)
    pub level: DiagnosticLevel,
    /// Standard rustc error code if present (e.g. "E0382", "E0597")
    pub code: Option<String>,
    /// Primary error or warning message
    pub message: String,
    /// Location in source code where issue originated
    pub span: Option<DiagnosticSpan>,
    /// Verbatim text slice for this diagnostic block
    pub raw_text: String,
}

impl CompilerDiagnostic {
    /// Returns the rustc error code as string slice if available.
    pub fn error_code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns true if this diagnostic is an error.
    pub fn is_error(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Error)
    }

    /// Returns true if this diagnostic is a warning.
    pub fn is_warning(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Warning)
    }

    /// Returns a concise one-line summary of the diagnostic.
    pub fn summary(&self) -> String {
        let code_str = match &self.code {
            Some(c) => format!("[{c}]"),
            None => String::new(),
        };
        let span_str = match &self.span {
            Some(s) => format!(" at {s}"),
            None => String::new(),
        };
        format!("{}{code_str}: {}{span_str}", self.level, self.message)
    }
}

static HEADER_REGEX: OnceLock<Regex> = OnceLock::new();
static SPAN_REGEX: OnceLock<Regex> = OnceLock::new();

fn header_regex() -> &'static Regex {
    HEADER_REGEX.get_or_init(|| {
        Regex::new(r"(?m)^(error|warning|note|help)(?:\[(E\d{4})\])?: (.*)$")
            .expect("Valid header regex pattern")
    })
}

fn span_regex() -> &'static Regex {
    SPAN_REGEX.get_or_init(|| {
        Regex::new(r"^\s*-->\s+([^:\r\n]+):(\d+):(\d+)").expect("Valid span regex pattern")
    })
}

/// Parser for rustc compiler diagnostics from stderr text.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiagnosticParser;

impl DiagnosticParser {
    /// Parses all compiler diagnostics from rustc stderr output.
    pub fn parse(stderr: &str) -> Vec<CompilerDiagnostic> {
        let re_header = header_regex();
        let re_span = span_regex();

        let mut matches: Vec<(usize, usize, DiagnosticLevel, Option<String>, String)> = Vec::new();

        for cap in re_header.captures_iter(stderr) {
            let m = cap.get(0).expect("capture group 0");
            let level = match &cap[1] {
                "error" => DiagnosticLevel::Error,
                "warning" => DiagnosticLevel::Warning,
                "note" => DiagnosticLevel::Note,
                "help" => DiagnosticLevel::Help,
                _ => continue,
            };
            let code = cap.get(2).map(|c| c.as_str().to_string());
            let message = cap[3].trim().to_string();
            matches.push((m.start(), m.end(), level, code, message));
        }

        if matches.is_empty() {
            return Vec::new();
        }

        let mut diagnostics = Vec::with_capacity(matches.len());
        for i in 0..matches.len() {
            let (start, _end, level, code, message) = matches[i].clone();
            let next_start = if i + 1 < matches.len() {
                matches[i + 1].0
            } else {
                stderr.len()
            };

            let block = &stderr[start..next_start];

            // Extract source span if available in block
            let mut span = None;
            for line in block.lines() {
                if let Some(span_cap) = re_span.captures(line) {
                    if let (Some(file), Some(line_num), Some(col_num)) =
                        (span_cap.get(1), span_cap.get(2), span_cap.get(3))
                    {
                        if let (Ok(l), Ok(c)) = (
                            line_num.as_str().parse::<usize>(),
                            col_num.as_str().parse::<usize>(),
                        ) {
                            span = Some(DiagnosticSpan {
                                file: file.as_str().trim().to_string(),
                                line: l,
                                column: c,
                            });
                            break;
                        }
                    }
                }
            }

            diagnostics.push(CompilerDiagnostic {
                level,
                code,
                message,
                span,
                raw_text: block.trim_end().to_string(),
            });
        }

        diagnostics
    }

    /// Extracts only error codes (e.g. ["E0382", "E0597"]) from compiler output.
    pub fn extract_error_codes(stderr: &str) -> Vec<String> {
        Self::parse(stderr)
            .into_iter()
            .filter_map(|d| d.code)
            .collect()
    }
}
