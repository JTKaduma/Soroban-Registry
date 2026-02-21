// api/src/regression_service.rs
// Background service for automated regression testing on deployments

use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::regression_engine::RegressionEngine;

/// Background task that monitors for new deployments and triggers regression tests
pub async fn run_regression_monitor(pool: PgPool) {
    info!("Starting regression testing monitor");

    let mut check_interval = interval(Duration::from_secs(60)); // Check every minute

    loop {
        check_interval.tick().await;

        if let Err(e) = check_and_run_tests(&pool).await {
            error!("Error in regression monitor: {}", e);
        }
    }
}

async fn check_and_run_tests(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Find deployments that need regression testing
    // Look for deployments in 'testing' status that haven't been tested yet
    let deployments: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
        r#"
        SELECT 
            cd.id as deployment_id,
            cd.contract_id,
            COALESCE(cv.version, '1.0.0') as version
        FROM contract_deployments cd
        LEFT JOIN contract_versions cv ON cv.contract_id = cd.contract_id
        WHERE cd.status = 'testing'
        AND NOT EXISTS (
            SELECT 1 FROM regression_test_runs rtr
            WHERE rtr.deployment_id = cd.id
            AND rtr.started_at > cd.deployed_at
        )
        ORDER BY cd.deployed_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;

    if deployments.is_empty() {
        return Ok(());
    }

    info!("Found {} deployments requiring regression tests", deployments.len());

    for (deployment_id, contract_id, version) in deployments {
        // Check if contract has auto-run test suites
        let suites: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM regression_test_suites 
             WHERE contract_id = $1 AND is_active = TRUE AND auto_run_on_deploy = TRUE",
        )
        .bind(contract_id)
        .fetch_all(pool)
        .await?;

        if suites.is_empty() {
            info!(
                "No auto-run test suites for contract {}, skipping",
                contract_id
            );
            continue;
        }

        info!(
            "Running {} test suites for contract {} version {}",
            suites.len(),
            contract_id,
            version
        );

        let engine = RegressionEngine::new(pool.clone());

        for (suite_name,) in suites {
            match engine
                .run_test_suite(
                    contract_id,
                    version.clone(),
                    suite_name.clone(),
                    "automated".to_string(),
                    Some(deployment_id),
                )
                .await
            {
                Ok(results) => {
                    let regressions = results.iter().filter(|r| r.regression_detected).count();
                    if regressions > 0 {
                        warn!(
                            "Detected {} regressions in suite {} for contract {}",
                            regressions, suite_name, contract_id
                        );
                    } else {
                        info!(
                            "All tests passed in suite {} for contract {}",
                            suite_name, contract_id
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to run test suite {} for contract {}: {}",
                        suite_name, contract_id, e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Scheduled task to calculate regression statistics for all contracts
pub async fn run_statistics_calculator(pool: PgPool) {
    info!("Starting regression statistics calculator");

    let mut calc_interval = interval(Duration::from_secs(3600)); // Run every hour

    loop {
        calc_interval.tick().await;

        if let Err(e) = calculate_all_statistics(&pool).await {
            error!("Error calculating statistics: {}", e);
        }
    }
}

async fn calculate_all_statistics(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("Calculating regression statistics for all contracts");

    // Get all contracts with regression tests
    let contracts: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT DISTINCT contract_id FROM regression_test_runs 
         WHERE started_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_all(pool)
    .await?;

    let period_end = chrono::Utc::now();
    let period_start = period_end - chrono::Duration::days(30);

    for (contract_id,) in contracts {
        match sqlx::query("SELECT calculate_regression_statistics($1, $2, $3)")
            .bind(contract_id)
            .bind(period_start)
            .bind(period_end)
            .execute(pool)
            .await
        {
            Ok(_) => {
                info!("Calculated statistics for contract {}", contract_id);
            }
            Err(e) => {
                error!(
                    "Failed to calculate statistics for contract {}: {}",
                    contract_id, e
                );
            }
        }
    }

    Ok(())
}
