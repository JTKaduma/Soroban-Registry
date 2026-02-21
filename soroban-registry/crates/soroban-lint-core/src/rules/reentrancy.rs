use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct ReentrancyRule;

impl LintRule for ReentrancyRule {
    fn rule_id(&self) -> &'static str {
        "reentrancy"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = ReentrancyVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }
}

struct ReentrancyVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl ReentrancyVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for ReentrancyVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let code_str = quote::quote!(#node).to_string();

        // Check for cross-contract calls before state writes
        let has_contract_call = code_str.contains("invoke") || code_str.contains("call");
        let has_state_write = code_str.contains("set") || code_str.contains("write");

        if has_contract_call && has_state_write {
            let diag = Diagnostic::new(
                "reentrancy",
                Severity::Error,
                "Potential reentrancy vulnerability: cross-contract call before state modification",
                &self.file,
                1,
                0,
            )
            .with_suggestion("Perform state updates before external calls (Checks-Effects-Interactions pattern)");

            self.diagnostics.push(diag);
        }

        syn::visit::visit_item_fn(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_created() {
        let rule = ReentrancyRule;
        assert_eq!(rule.rule_id(), "reentrancy");
    }
}
