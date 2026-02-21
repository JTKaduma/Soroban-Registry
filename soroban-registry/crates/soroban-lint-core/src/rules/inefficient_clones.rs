use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct InefficientClonesRule;

impl LintRule for InefficientClonesRule {
    fn rule_id(&self) -> &'static str {
        "inefficient_clones"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = InefficientClonesVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }

    fn supports_fix(&self) -> bool {
        true
    }
}

struct InefficientClonesVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl InefficientClonesVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for InefficientClonesVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if let syn::Expr::MethodCall(method_call) = node {
            if method_call.method == "clone" {
                let code_str = quote::quote!(#node).to_string();
                // Check for redundant clones
                if code_str.contains("clone().clone()") || code_str.contains("clone().as_ref()") {
                    let diag = Diagnostic::new(
                        "inefficient_clones",
                        Severity::Warning,
                        "Redundant clone() call detected",
                        &self.file,
                        1,
                        0,
                    )
                    .with_suggestion("Remove unnecessary clone or use references instead")
                    .with_fix("Replace redundant clone with reference or single clone");

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
        let rule = InefficientClonesRule;
        assert_eq!(rule.rule_id(), "inefficient_clones");
        assert!(rule.supports_fix());
    }
}
