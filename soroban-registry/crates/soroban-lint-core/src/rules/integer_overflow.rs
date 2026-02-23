use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct IntegerOverflowRule;

impl LintRule for IntegerOverflowRule {
    fn rule_id(&self) -> &'static str {
        "integer_overflow"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = IntegerOverflowVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct IntegerOverflowVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl IntegerOverflowVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for IntegerOverflowVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Binary(bin_expr) = node {
            match bin_expr.op {
                syn::BinOp::Add(_) | syn::BinOp::Sub(_) | syn::BinOp::Mul(_) => {
                    // Check if operands are integer types
                    let diag = Diagnostic::new(
                        "integer_overflow",
                        Severity::Error,
                        "Unchecked arithmetic operation on integers - use checked_add/sub/mul",
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion("Use checked_add(), checked_sub(), or checked_mul()");

                    // Only report if not already using checked variant
                    let expr_str = quote::quote!(#bin_expr).to_string();
                    if !expr_str.contains("checked_") {
                        self.diagnostics.push(diag);
                    }
                }
                _ => {}
            }
        }
        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = IntegerOverflowRule;
        assert_eq!(rule.rule_id(), "integer_overflow");
        assert_eq!(rule.default_severity(), Severity::Error);
    }
}
