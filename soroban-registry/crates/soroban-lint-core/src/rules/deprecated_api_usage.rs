use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct DeprecatedApiUsageRule;

impl LintRule for DeprecatedApiUsageRule {
    fn rule_id(&self) -> &'static str {
        "deprecated_api_usage"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = DeprecatedApiVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct DeprecatedApiVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl DeprecatedApiVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for DeprecatedApiVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let code_str = quote::quote!(#node).to_string();

        // Check for common deprecated API patterns
        let deprecated_apis = vec![
            ("invoke", "deprecated"),
            ("call_me", "deprecated"),
            ("exec", "use invoke instead"),
        ];

        for (api, _message) in deprecated_apis {
            if code_str.contains(api) {
                let diag = Diagnostic::new(
                    "deprecated_api_usage",
                    Severity::Warning,
                    format!("Deprecated API usage detected: {}", api),
                    &self.file,
                    1,
                    0,
                )
                .with_suggestion("Check Soroban SDK documentation for updated API");

                self.diagnostics.push(diag);
            }
        }

        syn::visit::visit_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = DeprecatedApiUsageRule;
        assert_eq!(rule.rule_id(), "deprecated_api_usage");
    }
}
