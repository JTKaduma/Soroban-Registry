use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct DirectStorageClearRule;

impl LintRule for DirectStorageClearRule {
    fn rule_id(&self) -> &'static str {
        "direct_storage_clear"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = DirectStorageClearVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct DirectStorageClearVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl DirectStorageClearVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for DirectStorageClearVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::MethodCall(method_call) = node {
            if method_call.method == "remove" || method_call.method == "clear" {
                let code_str = quote::quote!(#node).to_string();
                if code_str.contains("storage()") && code_str.contains("persistent()") {
                    let diag = Diagnostic::new(
                        "direct_storage_clear",
                        Severity::Warning,
                        "Direct storage clear/remove without validation",
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion("Verify keys and conditions before clearing storage");

                    self.diagnostics.push(diag);
                }
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
        let rule = DirectStorageClearRule;
        assert_eq!(rule.rule_id(), "direct_storage_clear");
    }
}
