// api/src/trust.rs
//
// Contract Trust Scoring Engine
//
// ── Score breakdown (max 100 points) ────────────────────────────────────────
//
//  Factor                  Weight   Description
//  ──────────────────────  ──────   ────────────────────────────────────────
//  Verification status       25 pt  +25 if is_verified = true
//  Audit quality             35 pt  latest audit overall_score × 0.35
//  Usage / adoption          20 pt  deployments + interactions, capped at 20
//  Contract age              10 pt  days since created_at, capped at 10
//  No critical vulns         10 pt  −10 per unresolved critical audit failure
//
// ── Trust tiers ─────────────────────────────────────────────────────────────
//
//  Score    Badge
//  ───────  ────────
//  90–100   Platinum
//  75–89    Gold
//  50–74    Silver
//   0–49    Bronze
//
// All weights are defined as constants so they are easy to audit and adjust.

use chrono::Utc;
use serde::Serialize;

// ── Weight constants ──────────────────────────────────────────────────────────

/// Maximum points awarded for on-chain verification
pub const WEIGHT_VERIFIED: f64 = 25.0;

/// Maximum points from audit quality (latest audit score × this fraction)
pub const WEIGHT_AUDIT: f64 = 35.0;

/// Maximum points from usage/adoption signals
pub const WEIGHT_USAGE: f64 = 20.0;

/// Maximum points from contract age
pub const WEIGHT_AGE: f64 = 10.0;

/// Maximum points from having no critical vulnerabilities
pub const WEIGHT_NO_VULNS: f64 = 10.0;

/// Number of deployments needed to earn full usage points
const USAGE_DEPLOYMENT_CAP: f64 = 50.0;

/// Number of interactions needed to contribute to usage points
const USAGE_INTERACTION_CAP: f64 = 500.0;

/// Days of age needed to earn full age points
const AGE_DAYS_CAP: f64 = 180.0;

// ── Input data ────────────────────────────────────────────────────────────────

/// Raw data collected from the DB before scoring
pub struct TrustInput {
    /// Whether the contract is verified on-chain
    pub is_verified: bool,

    /// Overall score (0–100) from the latest security audit, if any
    pub latest_audit_score: Option<f64>,

    /// Total number of deployments recorded in analytics
    pub total_deployments: i64,

    /// Total interactions recorded in analytics
    pub total_interactions: i64,

    /// Contract creation timestamp (used to compute age)
    pub created_at: chrono::DateTime<Utc>,

    /// Number of unresolved critical-severity audit check failures
    pub unresolved_critical_vulns: i64,
}

// ── Output types ──────────────────────────────────────────────────────────────

/// One factor contributing to the overall trust score
#[derive(Debug, Serialize)]
pub struct TrustFactor {
    /// Human-readable factor name
    pub name: &'static str,
    /// Points earned for this factor
    pub points_earned: f64,
    /// Maximum possible points for this factor
    pub points_max: f64,
    /// Plain-English explanation of why this score was given
    pub explanation: String,
}

/// Full trust score response
#[derive(Debug, Serialize)]
pub struct TrustScore {
    /// 0–100 composite trust score
    pub score: f64,
    /// Display badge (Platinum / Gold / Silver / Bronze)
    pub badge: &'static str,
    /// Emoji badge (for CLI / UI display)
    pub badge_icon: &'static str,
    /// Individual factor breakdown
    pub factors: Vec<TrustFactor>,
    /// Human-readable summary
    pub summary: String,
}

// ── Badge assignment ──────────────────────────────────────────────────────────

/// Map a numeric score to a trust tier badge.
///
/// Tiers:
/// - Platinum : 90–100
/// - Gold     : 75–89
/// - Silver   : 50–74
/// - Bronze   :  0–49
pub fn trust_badge(score: f64) -> (&'static str, &'static str) {
    match score as u32 {
        90..=100 => ("Platinum", "🏆"),
        75..=89  => ("Gold",     "🥇"),
        50..=74  => ("Silver",   "🥈"),
        _        => ("Bronze",   "🥉"),
    }
}

// ── Scoring engine ────────────────────────────────────────────────────────────

/// Compute the composite trust score from the collected input signals.
///
/// Returns a fully-populated [`TrustScore`] with per-factor breakdown.
pub fn compute_trust_score(input: &TrustInput) -> TrustScore {
    let mut factors: Vec<TrustFactor> = Vec::with_capacity(5);
    let mut total = 0.0f64;

    // ── Factor 1: Verification status ────────────────────────────────────────
    let verification_points = if input.is_verified { WEIGHT_VERIFIED } else { 0.0 };
    total += verification_points;
    factors.push(TrustFactor {
        name: "Verification Status",
        points_earned: verification_points,
        points_max: WEIGHT_VERIFIED,
        explanation: if input.is_verified {
            "Contract source code has been verified on-chain.".into()
        } else {
            "Contract is not yet verified. Submit source code to earn these points.".into()
        },
    });

    // ── Factor 2: Audit quality ───────────────────────────────────────────────
    let audit_points = match input.latest_audit_score {
        Some(s) => (s / 100.0) * WEIGHT_AUDIT,
        None    => 0.0,
    };
    total += audit_points;
    factors.push(TrustFactor {
        name: "Audit Quality",
        points_earned: audit_points,
        points_max: WEIGHT_AUDIT,
        explanation: match input.latest_audit_score {
            Some(s) => format!(
                "Latest security audit scored {:.1}/100. Audit score contributes up to {:.0} trust points.",
                s, WEIGHT_AUDIT
            ),
            None => "No security audit found. Complete an audit to earn up to 35 points.".into(),
        },
    });

    // ── Factor 3: Usage / adoption ────────────────────────────────────────────
    // Blend deployments (weighted 60%) and interactions (weighted 40%), each capped
    let deploy_ratio  = (input.total_deployments  as f64 / USAGE_DEPLOYMENT_CAP).min(1.0);
    let interact_ratio = (input.total_interactions as f64 / USAGE_INTERACTION_CAP).min(1.0);
    let usage_points  = (deploy_ratio * 0.6 + interact_ratio * 0.4) * WEIGHT_USAGE;
    total += usage_points;
    factors.push(TrustFactor {
        name: "Usage & Adoption",
        points_earned: usage_points,
        points_max: WEIGHT_USAGE,
        explanation: format!(
            "{} deployments and {} interactions recorded. Full marks at {} deployments / {} interactions.",
            input.total_deployments,
            input.total_interactions,
            USAGE_DEPLOYMENT_CAP as i64,
            USAGE_INTERACTION_CAP as i64,
        ),
    });

    // ── Factor 4: Contract age ────────────────────────────────────────────────
    let age_days = (Utc::now() - input.created_at).num_days().max(0) as f64;
    let age_points = (age_days / AGE_DAYS_CAP).min(1.0) * WEIGHT_AGE;
    total += age_points;
    factors.push(TrustFactor {
        name: "Contract Age",
        points_earned: age_points,
        points_max: WEIGHT_AGE,
        explanation: format!(
            "Contract is {:.0} days old. Full age points awarded after {} days.",
            age_days, AGE_DAYS_CAP as i64,
        ),
    });

    // ── Factor 5: No critical vulnerabilities ─────────────────────────────────
    // Each unresolved critical vuln deducts from this factor (floored at 0)
    let vuln_penalty = (input.unresolved_critical_vulns as f64 * 5.0).min(WEIGHT_NO_VULNS);
    let vuln_points  = (WEIGHT_NO_VULNS - vuln_penalty).max(0.0);
    total += vuln_points;
    factors.push(TrustFactor {
        name: "Vulnerability Status",
        points_earned: vuln_points,
        points_max: WEIGHT_NO_VULNS,
        explanation: if input.unresolved_critical_vulns == 0 {
            "No unresolved critical vulnerabilities detected.".into()
        } else {
            format!(
                "{} unresolved critical vulnerability/vulnerabilities found. Each deducts 5 points.",
                input.unresolved_critical_vulns
            )
        },
    });

    // ── Assemble result ───────────────────────────────────────────────────────
    let score = total.clamp(0.0, 100.0);
    let (badge, badge_icon) = trust_badge(score);

    let summary = format!(
        "{} {} — Trust score {:.0}/100. {}",
        badge_icon,
        badge,
        score,
        match badge {
            "Platinum" => "Highly trusted contract with strong signals across all factors.",
            "Gold"     => "Well-established contract. Minor improvements possible.",
            "Silver"   => "Moderate trust. Consider getting verified and audited.",
            _          => "Low trust signals. Verification and auditing recommended.",
        }
    );

    TrustScore { score, badge, badge_icon, factors, summary }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> TrustInput {
        TrustInput {
            is_verified: false,
            latest_audit_score: None,
            total_deployments: 0,
            total_interactions: 0,
            created_at: Utc::now(),
            unresolved_critical_vulns: 0,
        }
    }

    #[test]
    fn zero_input_scores_zero_plus_age() {
        let score = compute_trust_score(&base_input());
        // Only age can be > 0 when created_at is now — but it rounds to ~0
        assert!(score.score < 5.0);
    }

    #[test]
    fn verified_adds_25_points() {
        let input = TrustInput { is_verified: true, ..base_input() };
        let score = compute_trust_score(&input);
        let v = score.factors.iter().find(|f| f.name == "Verification Status").unwrap();
        assert_eq!(v.points_earned, 25.0);
    }

    #[test]
    fn perfect_audit_adds_35_points() {
        let input = TrustInput { latest_audit_score: Some(100.0), ..base_input() };
        let score = compute_trust_score(&input);
        let a = score.factors.iter().find(|f| f.name == "Audit Quality").unwrap();
        assert!((a.points_earned - 35.0).abs() < 0.01);
    }

    #[test]
    fn critical_vulns_reduce_vuln_factor() {
        let input = TrustInput { unresolved_critical_vulns: 2, ..base_input() };
        let score = compute_trust_score(&input);
        let v = score.factors.iter().find(|f| f.name == "Vulnerability Status").unwrap();
        assert_eq!(v.points_earned, 0.0); // 2 × 5 = 10, fully consumed
    }

    #[test]
    fn score_clamped_at_100() {
        let input = TrustInput {
            is_verified: true,
            latest_audit_score: Some(100.0),
            total_deployments: 1000,
            total_interactions: 10000,
            created_at: Utc::now() - chrono::Duration::days(365),
            unresolved_critical_vulns: 0,
        };
        let score = compute_trust_score(&input);
        assert!(score.score <= 100.0);
    }

    #[test]
    fn badge_boundaries() {
        assert_eq!(trust_badge(100.0).0, "Platinum");
        assert_eq!(trust_badge(90.0).0,  "Platinum");
        assert_eq!(trust_badge(89.0).0,  "Gold");
        assert_eq!(trust_badge(75.0).0,  "Gold");
        assert_eq!(trust_badge(74.0).0,  "Silver");
        assert_eq!(trust_badge(50.0).0,  "Silver");
        assert_eq!(trust_badge(49.0).0,  "Bronze");
        assert_eq!(trust_badge(0.0).0,   "Bronze");
    }

    #[test]
    fn factors_count_is_five() {
        let score = compute_trust_score(&base_input());
        assert_eq!(score.factors.len(), 5);
    }
}