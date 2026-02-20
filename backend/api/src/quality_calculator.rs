// backend/api/src/quality_calculator.rs
//
// Pure computation — no DB calls, no async.
// Takes source code + test output → returns all metric structs.

use shared::{
    CodeMetrics, DocMetrics, QualityScoreBreakdown, QualityWeights,
    SecurityMetrics, TestMetrics,
};

// ─────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────

pub struct QualityCalculator;

impl QualityCalculator {
    /// Compute all metrics from raw source and optional test output.
    pub fn compute(
        source: &str,
        test_output: Option<&str>,
        audit_score: f64,
        critical_findings: i64,
        high_findings: i64,
        medium_findings: i64,
        low_findings: i64,
        is_verified: bool,
        has_formal_audit: bool,
    ) -> ComputedMetrics {
        let code = compute_code_metrics(source);
        let docs = compute_doc_metrics(source);
        let tests = compute_test_metrics(source, test_output);
        let security = SecurityMetrics {
            audit_score,
            critical_findings,
            high_findings,
            medium_findings,
            low_findings,
            is_verified,
            has_formal_audit,
        };

        ComputedMetrics { code, docs, tests, security }
    }

    /// Aggregate per-dimension metrics into 0–100 scores.
    pub fn score(metrics: &ComputedMetrics, weights: &QualityWeights) -> QualityScoreBreakdown {
        let code_score = score_code(&metrics.code);
        let test_score = score_tests(&metrics.tests);
        let doc_score = score_docs(&metrics.docs);
        let security_score = score_security(&metrics.security);

        let overall_score = code_score * weights.code
            + test_score * weights.tests
            + doc_score * weights.docs
            + security_score * weights.security;

        QualityScoreBreakdown {
            code_score: round2(code_score),
            test_score: round2(test_score),
            doc_score: round2(doc_score),
            security_score: round2(security_score),
            overall_score: round2(overall_score),
        }
    }
}

/// All computed metrics in one bundle
pub struct ComputedMetrics {
    pub code: CodeMetrics,
    pub docs: DocMetrics,
    pub tests: TestMetrics,
    pub security: SecurityMetrics,
}

// ─────────────────────────────────────────────────────────
// Code metrics
// ─────────────────────────────────────────────────────────

fn compute_code_metrics(source: &str) -> CodeMetrics {
    let mut lines_of_code = 0i64;
    let mut blank_lines = 0i64;
    let mut comment_lines = 0i64;
    let mut function_count = 0i64;
    let mut function_line_totals = 0i64;
    let mut deeply_nested_count = 0i64;

    let mut current_depth: i32 = 0;
    let mut max_depth_in_fn: i32 = 0;
    let mut in_function = false;
    let mut fn_line_count = 0i64;
    let mut complexities: Vec<i64> = Vec::new();
    let mut current_fn_complexity = 1i64; // base = 1 per function

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            blank_lines += 1;
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("/*") {
            comment_lines += 1;
            continue;
        }

        lines_of_code += 1;

        // Function detection (Rust style)
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("async fn ")
        {
            if in_function && function_count > 0 {
                complexities.push(current_fn_complexity);
                function_line_totals += fn_line_count;
            }
            function_count += 1;
            in_function = true;
            fn_line_count = 0;
            current_fn_complexity = 1;
            max_depth_in_fn = 0;
        }

        if in_function {
            fn_line_count += 1;
        }

        // Brace depth tracking
        for ch in trimmed.chars() {
            match ch {
                '{' => {
                    current_depth += 1;
                    if current_depth > max_depth_in_fn {
                        max_depth_in_fn = current_depth;
                    }
                }
                '}' => {
                    current_depth -= 1;
                    if current_depth < 0 {
                        current_depth = 0;
                    }
                    // Function closed
                    if current_depth == 0 && in_function {
                        complexities.push(current_fn_complexity);
                        function_line_totals += fn_line_count;
                        in_function = false;
                        fn_line_count = 0;
                        if max_depth_in_fn > 3 {
                            deeply_nested_count += 1;
                        }
                        max_depth_in_fn = 0;
                    }
                }
                _ => {}
            }
        }

        // Cyclomatic complexity: +1 per branch keyword
        for keyword in &["if ", "else if ", "match ", "while ", "for ", "loop ", "? ", "panic!"] {
            if trimmed.contains(keyword) {
                current_fn_complexity += 1;
            }
        }
    }

    // Flush last open function
    if in_function && function_count > 0 {
        complexities.push(current_fn_complexity);
        function_line_totals += fn_line_count;
    }

    let max_function_complexity = complexities.iter().copied().max().unwrap_or(0);
    let avg_complexity = if complexities.is_empty() {
        0.0
    } else {
        complexities.iter().sum::<i64>() as f64 / complexities.len() as f64
    };
    let avg_function_length = if function_count == 0 {
        0.0
    } else {
        function_line_totals as f64 / function_count as f64
    };

    CodeMetrics {
        lines_of_code,
        blank_lines,
        comment_lines,
        cyclomatic_complexity: round2(avg_complexity),
        max_function_complexity,
        function_count,
        avg_function_length: round2(avg_function_length),
        deeply_nested_count,
    }
}

// ─────────────────────────────────────────────────────────
// Documentation metrics
// ─────────────────────────────────────────────────────────

fn compute_doc_metrics(source: &str) -> DocMetrics {
    let mut pub_fns = 0usize;
    let mut documented_fns = 0usize;
    let mut pub_types = 0usize;
    let mut documented_types = 0usize;
    let mut example_count = 0i64;

    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Count pub fns and whether preceding line has a doc comment
        if trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
        {
            pub_fns += 1;
            if i > 0 {
                let prev = lines[i - 1].trim();
                if prev.starts_with("///") || prev.ends_with("*/") {
                    documented_fns += 1;
                }
            }
        }

        // Count pub structs/enums/traits
        if trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub trait ")
        {
            pub_types += 1;
            if i > 0 {
                let prev = lines[i - 1].trim();
                if prev.starts_with("///") || prev.ends_with("*/") {
                    documented_types += 1;
                }
            }
        }

        // Count # Example blocks in doc comments
        if trimmed.contains("# Example") || trimmed.contains("# Examples") {
            example_count += 1;
        }
    }

    let public_fn_doc_coverage = if pub_fns == 0 {
        1.0 // no public fns → vacuously complete
    } else {
        documented_fns as f64 / pub_fns as f64
    };

    let type_doc_coverage = if pub_types == 0 {
        1.0
    } else {
        documented_types as f64 / pub_types as f64
    };

    DocMetrics {
        public_fn_doc_coverage: round2(public_fn_doc_coverage),
        type_doc_coverage: round2(type_doc_coverage),
        // These can't be determined from source alone —
        // the caller (handler) should set them from filesystem checks
        has_readme: false,
        has_changelog: false,
        has_license: false,
        example_count,
    }
}

// ─────────────────────────────────────────────────────────
// Test metrics
// ─────────────────────────────────────────────────────────

fn compute_test_metrics(source: &str, test_output: Option<&str>) -> TestMetrics {
    let mut test_count = 0i64;
    let mut test_lines = 0i64;
    let mut in_test_module = false;
    let mut brace_depth: i32 = 0;
    let mut has_integration_tests = false;
    let mut has_property_tests = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.contains("#[cfg(test)]") || trimmed.contains("mod tests") {
            in_test_module = true;
        }
        if trimmed.starts_with("#[test]") {
            test_count += 1;
        }
        // proptest / quickcheck detection
        if trimmed.contains("proptest!") || trimmed.contains("#[quickcheck]") {
            has_property_tests = true;
        }
        // Integration test hint (tests/ directory references)
        if trimmed.contains("integration") || trimmed.contains("e2e") {
            has_integration_tests = true;
        }

        if in_test_module {
            test_lines += 1;
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            in_test_module = false;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Parse coverage from `cargo tarpaulin` or `cargo llvm-cov` output
    let (line_coverage, function_coverage, branch_coverage) =
        parse_coverage(test_output.unwrap_or(""));

    let source_lines = source.lines().count() as i64;
    let test_to_code_ratio = if source_lines == 0 {
        0.0
    } else {
        test_lines as f64 / source_lines as f64
    };

    TestMetrics {
        test_count,
        test_lines,
        line_coverage,
        function_coverage,
        branch_coverage,
        test_to_code_ratio: round2(test_to_code_ratio),
        has_integration_tests,
        has_property_tests,
    }
}

/// Parse coverage percentages from tarpaulin or llvm-cov stdout
fn parse_coverage(output: &str) -> (f64, f64, f64) {
    let mut line_cov = 0.0f64;
    let mut fn_cov = 0.0f64;
    let mut branch_cov = 0.0f64;

    for line in output.lines() {
        // tarpaulin:  "85.71% coverage, 60/70 lines covered"
        if line.contains("coverage") && line.contains("lines") {
            if let Some(pct) = extract_pct(line) {
                line_cov = pct;
            }
        }
        // llvm-cov:   "TOTAL   ... 85.71%   72.50%"
        if line.trim_start().starts_with("TOTAL") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                fn_cov = parse_pct(parts[parts.len() - 2]).unwrap_or(0.0);
                line_cov = parse_pct(parts[parts.len() - 1]).unwrap_or(line_cov);
            }
        }
        // branch coverage hint
        if line.contains("branch") {
            if let Some(pct) = extract_pct(line) {
                branch_cov = pct;
            }
        }
    }

    (round2(line_cov), round2(fn_cov), round2(branch_cov))
}

fn extract_pct(s: &str) -> Option<f64> {
    s.split_whitespace()
        .find(|t| t.ends_with('%'))
        .and_then(|t| parse_pct(t))
}

fn parse_pct(s: &str) -> Option<f64> {
    s.trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0)
}

// ─────────────────────────────────────────────────────────
// Scoring — each dimension → 0..100
// ─────────────────────────────────────────────────────────

/// Code score: penalise high complexity, long functions, deep nesting
fn score_code(m: &CodeMetrics) -> f64 {
    let mut score = 100.0f64;

    // Cyclomatic complexity penalty (ideal ≤ 5, bad > 15)
    score -= clamp((m.cyclomatic_complexity - 5.0).max(0.0) * 2.5, 0.0, 40.0);

    // Max complexity (a single very complex function is a big smell)
    score -= clamp((m.max_function_complexity as f64 - 10.0).max(0.0) * 1.5, 0.0, 20.0);

    // Deeply nested blocks
    score -= clamp(m.deeply_nested_count as f64 * 5.0, 0.0, 20.0);

    // Very long functions (ideal avg ≤ 30 lines)
    score -= clamp((m.avg_function_length - 30.0).max(0.0) * 0.5, 0.0, 20.0);

    clamp(score, 0.0, 100.0)
}

/// Test score: reward coverage and test volume
fn score_tests(m: &TestMetrics) -> f64 {
    let mut score = 0.0f64;

    // Line coverage: up to 60 points (100% = 60 pts)
    score += m.line_coverage * 60.0;

    // Function coverage: up to 20 points
    score += m.function_coverage * 20.0;

    // Test/code ratio: up to 10 points (ideal ≥ 0.5)
    score += clamp(m.test_to_code_ratio * 20.0, 0.0, 10.0);

    // Bonuses
    if m.has_integration_tests { score += 5.0; }
    if m.has_property_tests    { score += 5.0; }

    clamp(score, 0.0, 100.0)
}

/// Doc score: reward documented public surface
fn score_docs(m: &DocMetrics) -> f64 {
    let mut score = 0.0f64;

    // Public function doc coverage: up to 50 points
    score += m.public_fn_doc_coverage * 50.0;

    // Type doc coverage: up to 30 points
    score += m.type_doc_coverage * 30.0;

    // File-level docs: up to 20 points
    if m.has_readme    { score += 8.0; }
    if m.has_changelog { score += 6.0; }
    if m.has_license   { score += 6.0; }

    // Examples bonus (up to 10 extra)
    score += clamp(m.example_count as f64 * 2.0, 0.0, 10.0);

    clamp(score, 0.0, 100.0)
}

/// Security score: based on audit result + findings
fn score_security(m: &SecurityMetrics) -> f64 {
    let mut score = m.audit_score; // 0–100 from AuditRecord

    // Hard deductions for serious findings
    score -= m.critical_findings as f64 * 25.0;
    score -= m.high_findings     as f64 * 10.0;
    score -= m.medium_findings   as f64 * 3.0;
    score -= m.low_findings      as f64 * 1.0;

    // Bonuses
    if m.is_verified      { score += 5.0; }
    if m.has_formal_audit { score += 10.0; }

    clamp(score, 0.0, 100.0)
}

// ─────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────

fn clamp(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ─────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SOURCE: &str = r#"
/// A simple token contract
pub struct Token;

/// Transfer tokens between accounts
pub fn transfer(from: Address, to: Address, amount: u64) -> bool {
    if from == to {
        return false;
    }
    if amount == 0 {
        return false;
    }
    true
}

pub fn balance(account: Address) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_same_address() {
        assert!(!transfer(addr(), addr(), 100));
    }

    #[test]
    fn test_transfer_zero_amount() {
        assert!(!transfer(addr(), other(), 0));
    }
}
"#;

    #[test]
    fn test_code_metrics_basic() {
        let m = compute_code_metrics(SAMPLE_SOURCE);
        assert!(m.function_count >= 2, "should find at least transfer and balance");
        assert!(m.lines_of_code > 0);
    }

    #[test]
    fn test_doc_metrics_coverage() {
        let m = compute_doc_metrics(SAMPLE_SOURCE);
        // transfer is documented, balance is not → 50%
        assert!(m.public_fn_doc_coverage > 0.0);
        assert!(m.public_fn_doc_coverage <= 1.0);
    }

    #[test]
    fn test_test_metrics_count() {
        let m = compute_test_metrics(SAMPLE_SOURCE, None);
        assert_eq!(m.test_count, 2);
        assert!(m.test_lines > 0);
    }

    #[test]
    fn test_overall_score_range() {
        let metrics = QualityCalculator::compute(
            SAMPLE_SOURCE, None,
            75.0, 0, 1, 2, 3, true, false,
        );
        let weights = QualityWeights::default();
        let breakdown = QualityCalculator::score(&metrics, &weights);
        assert!(breakdown.overall_score >= 0.0);
        assert!(breakdown.overall_score <= 100.0);
    }

    #[test]
    fn test_weights_must_sum_to_one() {
        let good = QualityWeights::default();
        assert!(good.is_valid());
        let bad = QualityWeights { code: 0.5, tests: 0.5, docs: 0.5, security: 0.5 };
        assert!(!bad.is_valid());
    }

    #[test]
    fn test_badge_thresholds() {
        assert_eq!(QualityBadge::from_score(95.0), QualityBadge::Excellent);
        assert_eq!(QualityBadge::from_score(80.0), QualityBadge::Good);
        assert_eq!(QualityBadge::from_score(60.0), QualityBadge::Fair);
        assert_eq!(QualityBadge::from_score(30.0), QualityBadge::Poor);
        assert_eq!(QualityBadge::from_score(10.0), QualityBadge::Critical);
    }

    #[test]
    fn test_coverage_parsing_tarpaulin() {
        let output = "85.71% coverage, 60/70 lines covered";
        let (line, _, _) = parse_coverage(output);
        assert!((line - 0.8571).abs() < 0.001);
    }
}