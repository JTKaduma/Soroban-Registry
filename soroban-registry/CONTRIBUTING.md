# Contributing to soroban-registry

Thank you for your interest in contributing to soroban-registry! We welcome contributions of all kinds: bug reports, feature requests, documentation improvements, and code submissions.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo
- Git

### Development Setup

1. Clone the repository:

```bash
git clone https://github.com/stellar/soroban-registry.git
cd soroban-registry
```

2. Build the project:

```bash
cargo build
```

3. Run tests:

```bash
cargo test
```

4. Run the CLI:

```bash
cargo run --bin soroban-registry -- lint examples/
```

## Contributing Process

### Reporting Bugs

Please use GitHub Issues to report bugs. Include:

- A clear description of the bug
- Steps to reproduce
- Expected vs. actual behavior
- Your environment (OS, Rust version)
- Error messages and logs

### Proposing Features

Open a GitHub Discussion or Issue to propose new features. Include:

- Use case and motivation
- Proposed solution (if you have one)
- Alternatives you've considered
- Any potential drawbacks

### Submitting Code

1. Create a feature branch:

```bash
git checkout -b feature/your-feature-name
```

2. Make your changes and add tests:

```bash
# For new lint rules
cargo test -p soroban-lint-core
```

3. Ensure code quality:

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Run all tests
cargo test
```

4. Commit with clear messages:

```bash
git commit -m "Add description of changes"
```

5. Push and create a Pull Request:

```bash
git push origin feature/your-feature-name
```

## Adding New Lint Rules

To add a new lint rule:

### 1. Create the Rule File

Create a new file in `crates/soroban-lint-core/src/rules/your_rule.rs`:

```rust
use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::LintRule;
use syn::visit::Visit;

pub struct YourRuleName;

impl LintRule for YourRuleName {
    fn rule_id(&self) -> &'static str {
        "your_rule_id"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning  // or Error, Info
    }

    fn check(&self, file: &str, syntax: &syn::File) -> Vec<Diagnostic> {
        let mut visitor = YourRuleVisitor::new(file);
        visitor.visit_file(syntax);
        visitor.diagnostics
    }

    fn supports_fix(&self) -> bool {
        false  // set to true if auto-fixing is supported
    }
}

struct YourRuleVisitor {
    file: String,
    diagnostics: Vec<Diagnostic>,
}

impl YourRuleVisitor {
    fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for YourRuleVisitor {
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        // Implement detection logic
        syn::visit::visit_expr(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_issue() {
        let code = r#"
            // Code that should trigger the rule
        "#;
        let syntax: syn::File = syn::parse_str(code).unwrap();
        let rule = YourRuleName;
        let diags = rule.check("test.rs", &syntax);
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_no_false_positive() {
        let code = r#"
            // Code that should NOT trigger the rule
        "#;
        let syntax: syn::File = syn::parse_str(code).unwrap();
        let rule = YourRuleName;
        let diags = rule.check("test.rs", &syntax);
        assert!(diags.is_empty());
    }
}
```

### 2. Register the Rule

Add to `crates/soroban-lint-core/src/rules/mod.rs`:

```rust
pub mod your_rule;
```

And in `crates/soroban-lint-core/src/analyzer.rs`, add to the rules vector:

```rust
Box::new(crate::rules::your_rule::YourRuleName),
```

### 3. Update Documentation

Update the rules table in `README.md` with:

```markdown
| `your_rule_id` | Severity | Description |
```

### 4. Update Configuration

Add to `.soroban-lint.toml`:

```toml
your_rule_id = "warning"  # or "error", "info"
```

### 5. Run Tests

```bash
cargo test
```

## Code Style

We follow standard Rust conventions:

- Use `cargo fmt` to format code
- Follow clippy recommendations
- Write tests for all new functionality
- Add documentation for public APIs

## Pull Request Guidelines

- Link related issues
- Provide a clear description of changes
- Ensure all tests pass
- Keep PRs focused on a single feature or fix
- Respond promptly to review feedback

## Code of Conduct

We are committed to providing a welcoming and inspiring community for all. Please read and follow our Code of Conduct.

## Questions?

Feel free to open an issue or discussion if you have questions!

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.
