// Integration tests for soroban-registry

#[test]
fn test_analyzer_with_sample_contract() {
    use soroban_lint_core::Analyzer;
    
    let sample_code = r#"
        pub fn transfer() {
            let x = Some(5).unwrap();
        }
    "#;
    
    let syntax: syn::File = syn::parse_str(sample_code).unwrap();
    let analyzer = Analyzer::new();
    
    let diagnostics = analyzer.analyze_file("test.rs", sample_code).unwrap();
    
    // Should detect missing_error_handling
    assert!(diagnostics.iter().any(|d| d.rule_id == "missing_error_handling"));
}

#[test]
fn test_config_loading() {
    use soroban_lint_core::LintConfig;
    
    let config = LintConfig::default();
    assert_eq!(config.lint.level, "warning");
}

#[test]
fn test_severity_filtering() {
    use soroban_lint_core::{Analyzer, Diagnostic, Severity};
    
    let diags = vec![
        Diagnostic::new("rule1", Severity::Error, "msg1", "file.rs", 1, 0),
        Diagnostic::new("rule2", Severity::Warning, "msg2", "file.rs", 2, 0),
        Diagnostic::new("rule3", Severity::Info, "msg3", "file.rs", 3, 0),
    ];
    
    let filtered = Analyzer::filter_by_severity(diags, Severity::Error);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].severity, Severity::Error);
}

#[test]
fn test_rules_list() {
    use soroban_lint_core::Analyzer;
    
    let analyzer = Analyzer::new();
    let rules = analyzer.list_rules();
    
    // Should have at least 20 rules
    assert!(rules.len() >= 20, "Expected at least 20 rules, got {}", rules.len());
    
    // Should include specific rules
    let rule_ids: Vec<_> = rules.iter().map(|(id, _)| *id).collect();
    assert!(rule_ids.contains(&"missing_error_handling"));
    assert!(rule_ids.contains(&"unsafe_unwrap"));
    assert!(rule_ids.contains(&"panic_in_contract"));
}
