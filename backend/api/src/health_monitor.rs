use anyhow::Result;
use chrono::Utc;
use shared::{Contract, ContractHealth, ContractStats, HealthStatus};
use sqlx::PgPool;
use tokio::time;
use tracing::{error, info};

use crate::state::AppState;

/// Main loop for the health monitor background task
pub async fn run_health_monitor(state: AppState) {
    info!("Starting health monitor background task");
    
    // Run every 24 hours in production, but for demo/dev we can run it more frequently or on startup
    // For now, we'll run it on startup and then every hour
    let mut interval = time::interval(time::Duration::from_secs(3600));

    loop {
        interval.tick().await;
        info!("Running health checks...");

        if let Err(e) = perform_health_checks(&state.db).await {
            error!("Error performing health checks: {}", e);
        }
    }
}

async fn perform_health_checks(pool: &PgPool) -> Result<()> {
    // 1. Fetch all contracts
    let contracts: Vec<Contract> = sqlx::query_as("SELECT * FROM contracts")
        .fetch_all(pool)
        .await?;
        
    info!("Found {} contracts to check", contracts.len());

    for contract in contracts {
        // 2. Fetch stats (last activity)
        let stats: Option<ContractStats> = sqlx::query_as("SELECT * FROM contract_stats WHERE contract_id = $1")
            .bind(contract.id)
            .fetch_optional(pool)
            .await?;

        // 3. Fetch verification status (if not in contract struct, though it is)
        // contract.is_verified is available

        // 4. Calculate health score
        let health = calculate_health(&contract, stats.as_ref());

        // 5. Update database
        upsert_contract_health(pool, &health).await?;
    }

    info!("Health checks completed");
    Ok(())
}

fn calculate_health(contract: &Contract, stats: Option<&ContractStats>) -> ContractHealth {
    let mut score = 100;
    
    // Penalize for not being verified
    if !contract.is_verified {
        score -= 40;
    }

    // Penalize for inactivity (older than 30 days)
    let last_activity = stats
        .and_then(|s| s.last_interaction)
        .unwrap_or(contract.created_at);
        
    let days_since_activity = (Utc::now() - last_activity).num_days();
    
    if days_since_activity > 30 {
        score -= 20;
    }
    
    if days_since_activity > 90 {
        score -= 20;
    }

    // Placeholder for audit check (not implemented yet)
    // score -= 10; 

    // Ensure score is within 0-100
    score = score.max(0).min(100);

    let mut recommendations = Vec::new();

    let status = match score {
        80..=100 => HealthStatus::Healthy,
        50..=79 => HealthStatus::Warning,
        _ => {
            tracing::warn!(contract_id = %contract.id, score, "Contract health is critical");
            HealthStatus::Critical
        },
    };

    if !contract.is_verified {
        recommendations.push("Verify the contract source code to improve trust and health score.".to_string());
    }

    if days_since_activity > 90 {
        recommendations.push("Contract has been inactive for over 90 days. Consider engaging users or updating the contract.".to_string());
    } else if days_since_activity > 30 {
        recommendations.push("Contract has been inactive for over 30 days.".to_string());
    }

    if recommendations.is_empty() {
        recommendations.push("Contract is healthy and active. Keep it up!".to_string());
    }

    ContractHealth {
        contract_id: contract.id,
        status,
        last_activity,
        security_score: score / 2, // Placeholder logic
        audit_date: None,
        total_score: score,
        recommendations,
        updated_at: Utc::now(),
    }
}

async fn upsert_contract_health(pool: &PgPool, health: &ContractHealth) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO contract_health (contract_id, status, last_activity, security_score, audit_date, total_score, recommendations, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (contract_id) 
        DO UPDATE SET 
            status = EXCLUDED.status,
            last_activity = EXCLUDED.last_activity,
            security_score = EXCLUDED.security_score,
            audit_date = EXCLUDED.audit_date,
            total_score = EXCLUDED.total_score,
            recommendations = EXCLUDED.recommendations,
            updated_at = EXCLUDED.updated_at
        "#
    )
    .bind(health.contract_id)
    .bind(&health.status)
    .bind(health.last_activity)
    .bind(health.security_score)
    .bind(health.audit_date)
    .bind(health.total_score)
    .bind(&health.recommendations)
    .bind(health.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}
