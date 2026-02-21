use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct PanicInContractRule;

impl LintRule for PanicInContractRule {
    fn rule_id(&self) -> &'static str {
        "panic_in_contract"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = PanicVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct PanicVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl PanicVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for PanicVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::Macro(expr_macro) = node {
            if expr_macro.mac.path.is_ident("panic") {
                let diag = Diagnostic::new(
                    "panic_in_contract",
                    Severity::Error,
                    "panic! macro used in contract code - contract will trap",
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Use env.fail_with_error() or return Err() instead");

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
        let rule = PanicInContractRule;
        assert_eq!(rule.rule_id(), "panic_in_contract");
    }
}
