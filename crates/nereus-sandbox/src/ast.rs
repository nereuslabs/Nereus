//! AST-based static code analysis and pedagogical anti-pattern inspection.
//!
//! Copyright (c) 2026 Nereus Labs. Licensed under the MIT License.

use nereus_core::models::{PedagogicalSmell, SmellSeverity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprForLoop, ExprMethodCall, ExprUnsafe};

/// Visitor traversing the Rust AST and detecting educational code smells.
#[derive(Default)]
struct SmellVisitor {
    smells: Vec<PedagogicalSmell>,
}

impl<'ast> Visit<'ast> for SmellVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();
        let span = node.method.span();
        let line = span.start().line;
        let column = span.start().column;

        if method_name == "clone" {
            self.smells.push(PedagogicalSmell {
                rule_id: "CLONE_USAGE".to_string(),
                severity: SmellSeverity::Warning,
                line,
                column,
                message: "Explicit invocation of .clone() detected".to_string(),
                socratic_hint: "Consider whether data ownership must be duplicated here, or if a borrowed reference (&T, &str, or &[T]) would suffice.".to_string(),
            });
        } else if method_name == "unwrap" {
            self.smells.push(PedagogicalSmell {
                rule_id: "UNWRAP_USAGE".to_string(),
                severity: SmellSeverity::Warning,
                line,
                column,
                message: "Direct invocation of .unwrap() detected".to_string(),
                socratic_hint: "Can this potential failure be handled gracefully using the '?' operator or exhaustive pattern matching (match / if let)?".to_string(),
            });
        }

        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast ExprUnsafe) {
        let span = node.unsafe_token.span();
        let line = span.start().line;
        let column = span.start().column;

        self.smells.push(PedagogicalSmell {
            rule_id: "UNSAFE_BLOCK".to_string(),
            severity: SmellSeverity::Critical,
            line,
            column,
            message: "Unsafe memory block detected".to_string(),
            socratic_hint: "What safety invariants are being upheld here, and can this be implemented using safe standard library abstractions?".to_string(),
        });

        visit::visit_expr_unsafe(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast ExprForLoop) {
        // Inspect if iterating over a range like 0..v.len()
        if let Expr::Range(range) = &*node.expr {
            let is_zero_start = match &range.start {
                Some(start_expr) => {
                    if let Expr::Lit(lit) = &**start_expr {
                        if let syn::Lit::Int(int_lit) = &lit.lit {
                            int_lit.base10_digits() == "0"
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                None => true,
            };

            let is_len_end = match &range.end {
                Some(end_expr) => {
                    if let Expr::MethodCall(call) = &**end_expr {
                        call.method == "len"
                    } else {
                        false
                    }
                }
                None => false,
            };

            if is_zero_start && is_len_end {
                let span = node.for_token.span();
                self.smells.push(PedagogicalSmell {
                    rule_id: "INDEXED_FOR_LOOP".to_string(),
                    severity: SmellSeverity::Info,
                    line: span.start().line,
                    column: span.start().column,
                    message: "C-style index-based loop range '0..len()' detected".to_string(),
                    socratic_hint: "Rust iterators (.iter(), .iter_mut(), or .enumerate()) provide bounds-check-free, idiomatic traversal over collections.".to_string(),
                });
            }
        }

        visit::visit_expr_for_loop(self, node);
    }
}

/// Static AST Inspector parsing candidate Rust source code and collecting pedagogical smells.
pub struct AstInspector;

impl AstInspector {
    /// Parses the provided Rust code and evaluates educational anti-pattern rules.
    ///
    /// If the code contains syntax errors and cannot be parsed into an AST,
    /// returns an empty list, allowing the compiler to report syntax errors first.
    pub fn inspect(code: &str) -> Vec<PedagogicalSmell> {
        let Ok(file) = syn::parse_file(code) else {
            return Vec::new();
        };

        let mut visitor = SmellVisitor::default();
        visitor.visit_file(&file);
        visitor.smells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_inspect_detects_clone() {
        let code = r#"
            fn duplicate(s: String) -> String {
                let copy = s.clone();
                copy
            }
        "#;
        let smells = AstInspector::inspect(code);
        assert_eq!(smells.len(), 1);
        assert_eq!(smells[0].rule_id, "CLONE_USAGE");
        assert_eq!(smells[0].severity, SmellSeverity::Warning);
        assert_eq!(smells[0].line, 3);
    }

    #[test]
    fn test_ast_inspect_detects_unwrap() {
        let code = r#"
            fn parse_num(s: &str) -> i32 {
                s.parse::<i32>().unwrap()
            }
        "#;
        let smells = AstInspector::inspect(code);
        assert_eq!(smells.len(), 1);
        assert_eq!(smells[0].rule_id, "UNWRAP_USAGE");
        assert_eq!(smells[0].severity, SmellSeverity::Warning);
    }

    #[test]
    fn test_ast_inspect_detects_unsafe() {
        let code = r#"
            fn deref_raw(ptr: *const i32) -> i32 {
                unsafe { *ptr }
            }
        "#;
        let smells = AstInspector::inspect(code);
        assert_eq!(smells.len(), 1);
        assert_eq!(smells[0].rule_id, "UNSAFE_BLOCK");
        assert_eq!(smells[0].severity, SmellSeverity::Critical);
    }

    #[test]
    fn test_ast_inspect_detects_indexed_loop() {
        let code = r#"
            fn sum(v: &[i32]) -> i32 {
                let mut total = 0;
                for i in 0..v.len() {
                    total += v[i];
                }
                total
            }
        "#;
        let smells = AstInspector::inspect(code);
        assert_eq!(smells.len(), 1);
        assert_eq!(smells[0].rule_id, "INDEXED_FOR_LOOP");
        assert_eq!(smells[0].severity, SmellSeverity::Info);
    }

    #[test]
    fn test_ast_inspect_clean_idiomatic_code() {
        let code = r#"
            pub fn sum(v: &[i32]) -> i32 {
                v.iter().sum()
            }
        "#;
        let smells = AstInspector::inspect(code);
        assert!(smells.is_empty());
    }

    #[test]
    fn test_ast_inspect_syntax_error_graceful_fallback() {
        let code = "fn broken_syntax(";
        let smells = AstInspector::inspect(code);
        assert!(smells.is_empty());
    }
}
