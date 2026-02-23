use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct UnboundedLoopsRule;

impl LintRule for UnboundedLoopsRule {
    fn rule_id(&self) -> &'static str {
        "unbounded_loops"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = UnboundedLoopsVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct UnboundedLoopsVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl UnboundedLoopsVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for UnboundedLoopsVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Loop(_) = node {
            let code_str = quote::quote!(#node).to_string();
            if !code_str.contains("break") && !code_str.contains("return") {
                let diag = Diagnostic::new(
                    "unbounded_loops",
                    Severity::Warning,
                    "Unbounded loop detected - ensure explicit break condition",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Add explicit break condition or bounded iteration");

                self.diagnostics.push(diag);
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
        let rule = UnboundedLoopsRule;
        assert_eq!(rule.rule_id(), "unbounded_loops");
    }
}
