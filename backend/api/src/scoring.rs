// api/src/scoring.rs
// Scoring engine: weighted category scoring, badge assignment, and report generation

use std::collections::HashMap;
use crate::checklist::all_checks;
use crate::models::{AuditCheckRow, CategoryScore, CheckStatus, ChecklistItem, DetectionMethod, Severity};

pub fn severity_weight(sev: &Severity) -> f64 {
    match sev {
        Severity::Critical => 10.0,
        Severity::High     => 5.0,
        Severity::Medium   => 2.0,
        Severity::Low      => 1.0,
        Severity::Info     => 0.5,
    }
}

fn score_category(
    checks: &[&ChecklistItem],
    statuses: &HashMap<&str, &AuditCheckRow>,
) -> (f64, usize, usize, usize, usize) {
    let mut weighted_passed = 0.0f64;
    let mut weighted_total  = 0.0f64;
    let mut passed          = 0usize;
    let mut total           = 0usize;
    let mut failed_critical = 0usize;
    let mut failed_high     = 0usize;

    for item in checks {
        let status = statuses
            .get(item.id)
            .map(|r| &r.status)
            .unwrap_or(&CheckStatus::Pending);

        if *status == CheckStatus::NotApplicable { continue; }

        let w = severity_weight(&item.severity);
        weighted_total += w;
        total += 1;

        match status {
            CheckStatus::Passed => { weighted_passed += w; passed += 1; }
            CheckStatus::Failed => match item.severity {
                Severity::Critical => failed_critical += 1,
                Severity::High     => failed_high     += 1,
                _                  => {}
            },
            _ => {}
        }
    }

    let score = if weighted_total > 0.0 {
        (weighted_passed / weighted_total) * 100.0
    } else {
        100.0
    };

    (score, passed, total, failed_critical, failed_high)
}

pub fn calculate_scores(check_rows: &[AuditCheckRow]) -> (f64, Vec<CategoryScore>) {
    let all = all_checks();
    let status_map: HashMap<&str, &AuditCheckRow> = check_rows
        .iter()
        .map(|r| (r.check_id.as_str(), r))
        .collect();

    let mut by_category: HashMap<String, Vec<&ChecklistItem>> = HashMap::new();
    for item in &all {
        by_category.entry(item.category.to_string()).or_default().push(item);
    }

    let mut category_scores: Vec<CategoryScore> = Vec::new();
    let mut total_weighted_passed = 0.0f64;
    let mut total_weighted_total  = 0.0f64;

    for (category, items) in &by_category {
        let (score, passed, total, failed_critical, failed_high) =
            score_category(items, &status_map);

        for item in items {
            let s = status_map.get(item.id).map(|r| &r.status).unwrap_or(&CheckStatus::Pending);
            if *s == CheckStatus::NotApplicable { continue; }
            let w = severity_weight(&item.severity);
            total_weighted_total += w;
            if *s == CheckStatus::Passed { total_weighted_passed += w; }
        }

        category_scores.push(CategoryScore {
            category: category.clone(),
            score,
            passed,
            total,
            failed_critical,
            failed_high,
        });
    }

    category_scores.sort_by(|a, b| a.category.cmp(&b.category));

    let overall = if total_weighted_total > 0.0 {
        (total_weighted_passed / total_weighted_total) * 100.0
    } else {
        0.0
    };

    (overall, category_scores)
}

pub fn score_badge(score: f64) -> &'static str {
    match score as u32 {
        95..=100 => "🟢 EXCELLENT",
        80..=94  => "🔵 GOOD",
        60..=79  => "🟡 NEEDS IMPROVEMENT",
        40..=59  => "🟠 POOR",
        _        => "🔴 CRITICAL ISSUES",
    }
}

pub fn build_markdown_report(
    contract_name: &str,
    contract_id: &str,
    auditor: &str,
    audit_date: &str,
    overall_score: f64,
    check_rows: &[AuditCheckRow],
    category_scores: &[CategoryScore],
    include_descriptions: bool,
    failures_only: bool,
) -> String {
    let all = all_checks();
    // index by id as &str for lookup
    let meta: HashMap<&str, &ChecklistItem> = all.iter().map(|c| (c.id, c)).collect();
    let status_map: HashMap<&str, &AuditCheckRow> =
        check_rows.iter().map(|r| (r.check_id.as_str(), r)).collect();

    let mut md = String::with_capacity(8192);

    md.push_str("---\n");
    md.push_str(&format!("title: Security Audit — {}\n", contract_name));
    md.push_str(&format!("contract_id: {}\n", contract_id));
    md.push_str(&format!("auditor: {}\n", auditor));
    md.push_str(&format!("date: {}\n", audit_date));
    md.push_str(&format!("score: {:.1}%\n", overall_score));
    md.push_str("---\n\n");

    md.push_str(&format!("# Security Audit: {}\n\n", contract_name));
    md.push_str("| Field | Value |\n|-------|-------|\n");
    md.push_str(&format!("| **Contract ID** | `{}` |\n", contract_id));
    md.push_str(&format!("| **Auditor** | {} |\n", auditor));
    md.push_str(&format!("| **Audit Date** | {} |\n", audit_date));
    md.push_str(&format!("| **Overall Score** | **{:.1}%** |\n", overall_score));
    md.push_str(&format!("| **Badge** | {} |\n\n", score_badge(overall_score)));

    let filled = (overall_score / 100.0 * 20.0) as usize;
    let empty  = 20_usize.saturating_sub(filled);
    md.push_str(&format!("`[{}{}]` {:.1}%\n\n", "█".repeat(filled), "░".repeat(empty), overall_score));

    md.push_str("## Category Scores\n\n");
    md.push_str("| Category | Score | Passed | Total | Crit Fails | High Fails |\n");
    md.push_str("|----------|-------|--------|-------|------------|------------|\n");
    for cs in category_scores {
        let bar_filled = (cs.score / 100.0 * 10.0) as usize;
        let bar = format!("{}{}", "■".repeat(bar_filled), "□".repeat(10 - bar_filled));
        md.push_str(&format!(
            "| {} | `{}` {:.0}% | {} | {} | {} | {} |\n",
            cs.category, bar, cs.score, cs.passed, cs.total, cs.failed_critical, cs.failed_high,
        ));
    }
    md.push('\n');

    for severity in [Severity::Critical, Severity::High, Severity::Medium, Severity::Low, Severity::Info] {
        let mut section_items: Vec<(&ChecklistItem, &AuditCheckRow)> = all
            .iter()
            .filter(|item| item.severity == severity)
            .filter_map(|item| {
                let row = status_map.get(item.id)?;
                if failures_only && row.status == CheckStatus::Passed { return None; }
                Some((item, *row))
            })
            .collect();

        if section_items.is_empty() { continue; }
        section_items.sort_by(|a, b| a.0.id.cmp(&b.0.id));

        let sev_label = match severity {
            Severity::Critical => "🔴 Critical",
            Severity::High     => "🟠 High",
            Severity::Medium   => "🟡 Medium",
            Severity::Low      => "🔵 Low",
            Severity::Info     => "ℹ️ Info",
        };
        md.push_str(&format!("## {} Severity\n\n", sev_label));

        for (item, row) in &section_items {
            let status_icon = match row.status {
                CheckStatus::Passed        => "✅",
                CheckStatus::Failed        => "❌",
                CheckStatus::NotApplicable => "➖",
                CheckStatus::Pending       => "⏳",
            };
            let detect_type = match &item.detection {
                DetectionMethod::Automatic { .. }     => "🤖 Auto",
                DetectionMethod::SemiAutomatic { .. } => "🔍 Semi-Auto",
                DetectionMethod::Manual               => "👁️ Manual",
            };

            md.push_str(&format!(
                "### {} `{}` {} — {}\n\n**Category:** {}  \n",
                status_icon, item.id, detect_type, item.title, item.category
            ));

            if include_descriptions {
                md.push_str(&format!("**Description:** {}  \n", item.description));
                md.push_str(&format!("**Remediation:** {}  \n", item.remediation));
                if !item.references.is_empty() {
                    md.push_str("**References:**");
                    for r in &item.references {
                        md.push_str(&format!(" [link]({}) ", r));
                    }
                    md.push_str("  \n");
                }
            }
            if let Some(notes)    = &row.notes    { md.push_str(&format!("**Auditor Notes:** {}  \n", notes)); }
            if let Some(evidence) = &row.evidence { md.push_str(&format!("**Evidence:**\n```\n{}\n```\n", evidence)); }
            if row.auto_detected  { md.push_str("*Auto-detected by source analysis.*  \n"); }
            md.push('\n');
        }
    }

    let total   = check_rows.iter().filter(|r| r.status != CheckStatus::NotApplicable).count();
    let passed  = check_rows.iter().filter(|r| r.status == CheckStatus::Passed).count();
    let failed  = check_rows.iter().filter(|r| r.status == CheckStatus::Failed).count();
    let pending = check_rows.iter().filter(|r| r.status == CheckStatus::Pending).count();
    let na      = check_rows.iter().filter(|r| r.status == CheckStatus::NotApplicable).count();

    md.push_str("---\n\n## Audit Statistics\n\n");
    md.push_str("| Metric | Count |\n|--------|-------|\n");
    md.push_str(&format!("| Total applicable | {} |\n| ✅ Passed | {} |\n| ❌ Failed | {} |\n| ⏳ Pending | {} |\n| ➖ N/A | {} |\n", total, passed, failed, pending, na));
    md.push_str("\n---\n\n*Report generated by Soroban Security Audit*\n");

    md
}

// Suppress unused-import warning: meta is used in build_markdown_report via the HashMap
// but only indirectly — keep it as documentation of available lookup.
#[allow(dead_code)]
fn _use_meta() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_weights_ordered() {
        assert!(severity_weight(&Severity::Critical) > severity_weight(&Severity::High));
        assert!(severity_weight(&Severity::High)     > severity_weight(&Severity::Medium));
        assert!(severity_weight(&Severity::Medium)   > severity_weight(&Severity::Low));
    }

    #[test]
    fn badge_boundaries() {
        assert_eq!(score_badge(100.0), "🟢 EXCELLENT");
        assert_eq!(score_badge(80.0),  "🔵 GOOD");
        assert_eq!(score_badge(60.0),  "🟡 NEEDS IMPROVEMENT");
        assert_eq!(score_badge(40.0),  "🟠 POOR");
        assert_eq!(score_badge(20.0),  "🔴 CRITICAL ISSUES");
    }

    #[test]
    fn empty_checks_scores_zero() {
        let (score, _) = calculate_scores(&[]);
        assert_eq!(score, 0.0);
    }
}
