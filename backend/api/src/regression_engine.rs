// api/src/regression_engine.rs
// Core regression testing engine: runs tests, compares against baselines,
// detects regressions with configurable thresholds

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::benchmark_engine::{BenchmarkRunner, BenchmarkStats, IterationResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "test_status", rename_all = "lowercase")]
pub enum TestStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "regression_severity", rename_all = "lowercase")]
pub enum RegressionSeverity {
    None,
    Minor,
    Major,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestBaseline {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub test_suite_name: String,
    pub function_name: String,
    pub baseline_execution_time_ms: f64,
    pub baseline_memory_bytes: Option<i64>,
    pub baseline_cpu_instructions: Option<i64>,
    pub output_snapshot: serde_json::Value,
    pub output_hash: String,
    pub established_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub baseline_id: Option<Uuid>,
    pub test_suite_name: String,
    pub function_name: String,
    pub status: TestStatus,
    pub execution_time_ms: Option<f64>,
    pub memory_bytes: Option<i64>,
    pub output_data: Option<serde_json::Value>,
    pub output_hash: Option<String>,
    pub output_matches_baseline: Option<bool>,
    pub regression_detected: bool,
    pub regression_severity: RegressionSeverity,
    pub performance_degradation_percent: Option<f64>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub triggered_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub test_functions: serde_json::Value,
    pub performance_thresholds: Option<serde_json::Value>,
    pub auto_run_on_deploy: bool,
}

#[derive(Debug, Clone)]
pub struct RegressionThresholds {
    pub performance_degradation_minor: f64,    // e.g., 10%
    pub performance_degradation_major: f64,    // e.g., 25%
    pub performance_degradation_critical: f64, // e.g., 50%
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            performance_degradation_minor: 10.0,
            performance_degradation_major: 25.0,
            performance_degradation_critical: 50.0,
        }
    }
}

pub struct RegressionEngine {
    pool: PgPool,
    thresholds: RegressionThresholds,
}

impl RegressionEngine {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            thresholds: RegressionThresholds::default(),
        }
    }

    pub fn with_thresholds(pool: PgPool, thresholds: RegressionThresholds) -> Self {
        Self { pool, thresholds }
    }

    /// Establish a new baseline for a contract version
    pub async fn establish_baseline(
        &self,
        contract_id: Uuid,
        version: String,
        test_suite_name: String,
        function_name: String,
        output: serde_json::Value,
        established_by: Option<String>,
    ) -> Result<TestBaseline, sqlx::Error> {
        // Run benchmark to get performance baseline
        let runner = BenchmarkRunner::new(function_name.clone(), 50);
        let (results, stats) = runner.run();

        let output_hash = Self::hash_output(&output);

        // Deactivate previous baselines for this function
        sqlx::query(
            "UPDATE regression_test_baselines 
             SET is_active = FALSE 
             WHERE contract_id = $1 AND version = $2 
               AND test_suite_name = $3 AND function_name = $4",
        )
        .bind(contract_id)
        .bind(&version)
        .bind(&test_suite_name)
        .bind(&function_name)
        .execute(&self.pool)
        .await?;

        // Insert new baseline
        let baseline: TestBaseline = sqlx::query_as(
            r#"INSERT INTO regression_test_baselines (
                contract_id, version, test_suite_name, function_name,
                baseline_execution_time_ms, baseline_memory_bytes, 
                baseline_cpu_instructions, output_snapshot, output_hash,
                established_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING 
                id, contract_id, version, test_suite_name, function_name,
                baseline_execution_time_ms, baseline_memory_bytes,
                baseline_cpu_instructions, output_snapshot, output_hash,
                established_at"#,
        )
        .bind(contract_id)
        .bind(&version)
        .bind(&test_suite_name)
        .bind(&function_name)
        .bind(stats.avg_ms)
        .bind(results.first().and_then(|r| r.memory_bytes))
        .bind(results.first().and_then(|r| r.cpu_instructions))
        .bind(&output)
        .bind(&output_hash)
        .bind(established_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(baseline)
    }

    /// Run regression test against baseline
    pub async fn run_regression_test(
        &self,
        contract_id: Uuid,
        version: String,
        test_suite_name: String,
        function_name: String,
        triggered_by: String,
        deployment_id: Option<Uuid>,
    ) -> Result<TestRun, sqlx::Error> {
        // Create test run record
        let test_run_id = Uuid::new_v4();
        let started_at = Utc::now();

        sqlx::query(
            r#"INSERT INTO regression_test_runs (
                id, contract_id, version, test_suite_name, function_name,
                status, triggered_by, deployment_id, started_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(test_run_id)
        .bind(contract_id)
        .bind(&version)
        .bind(&test_suite_name)
        .bind(&function_name)
        .bind(TestStatus::Running)
        .bind(&triggered_by)
        .bind(deployment_id)
        .bind(started_at)
        .execute(&self.pool)
        .await?;

        // Fetch active baseline
        let baseline: Option<TestBaseline> = sqlx::query_as(
            r#"SELECT 
                id, contract_id, version, test_suite_name, function_name,
                baseline_execution_time_ms, baseline_memory_bytes,
                baseline_cpu_instructions, output_snapshot, output_hash,
                established_at
            FROM regression_test_baselines
            WHERE contract_id = $1 AND test_suite_name = $2 
              AND function_name = $3 AND is_active = TRUE
            ORDER BY established_at DESC
            LIMIT 1"#,
        )
        .bind(contract_id)
        .bind(&test_suite_name)
        .bind(&function_name)
        .fetch_optional(&self.pool)
        .await?;

        // Execute test
        let result = self.execute_test(&function_name).await;

        // Compare against baseline and detect regression
        let (status, regression_detected, severity, degradation, output_matches) =
            if let Some(ref baseline) = baseline {
                self.compare_with_baseline(&result, baseline)
            } else {
                // No baseline - test passes but no regression detection
                (TestStatus::Passed, false, RegressionSeverity::None, None, None)
            };

        let completed_at = Utc::now();
        let duration = (completed_at - started_at).num_seconds() as i32;

        // Update test run with results
        let test_run: TestRun = sqlx::query_as(
            r#"UPDATE regression_test_runs SET
                baseline_id = $1,
                status = $2,
                execution_time_ms = $3,
                memory_bytes = $4,
                cpu_instructions = $5,
                output_data = $6,
                output_hash = $7,
                output_matches_baseline = $8,
                regression_detected = $9,
                regression_severity = $10,
                performance_degradation_percent = $11,
                completed_at = $12,
                duration_seconds = $13,
                error_message = $14
            WHERE id = $15
            RETURNING 
                id, contract_id, version, baseline_id, test_suite_name,
                function_name, status as "status: TestStatus", 
                execution_time_ms, memory_bytes, output_data, output_hash,
                output_matches_baseline, regression_detected,
                regression_severity as "regression_severity: RegressionSeverity",
                performance_degradation_percent, started_at, completed_at,
                error_message, triggered_by"#,
        )
        .bind(baseline.as_ref().map(|b| b.id))
        .bind(&status)
        .bind(result.execution_time_ms)
        .bind(result.memory_bytes)
        .bind(result.cpu_instructions)
        .bind(&result.output)
        .bind(&result.output_hash)
        .bind(output_matches)
        .bind(regression_detected)
        .bind(&severity)
        .bind(degradation)
        .bind(completed_at)
        .bind(duration)
        .bind(result.error_message)
        .bind(test_run_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(test_run)
    }

    /// Run all tests in a suite
    pub async fn run_test_suite(
        &self,
        contract_id: Uuid,
        version: String,
        suite_name: String,
        triggered_by: String,
        deployment_id: Option<Uuid>,
    ) -> Result<Vec<TestRun>, sqlx::Error> {
        // Fetch test suite
        let suite: TestSuite = sqlx::query_as(
            r#"SELECT id, contract_id, name, description, test_functions,
                      performance_thresholds, auto_run_on_deploy
               FROM regression_test_suites
               WHERE contract_id = $1 AND name = $2 AND is_active = TRUE"#,
        )
        .bind(contract_id)
        .bind(&suite_name)
        .fetch_one(&self.pool)
        .await?;

        // Extract function names from test_functions JSON
        let functions: Vec<String> = if let Some(arr) = suite.test_functions.as_array() {
            arr.iter()
                .filter_map(|v| v.get("function").and_then(|f| f.as_str()))
                .map(String::from)
                .collect()
        } else {
            vec![]
        };

        let mut results = Vec::new();

        for function_name in functions {
            let test_run = self
                .run_regression_test(
                    contract_id,
                    version.clone(),
                    suite_name.clone(),
                    function_name,
                    triggered_by.clone(),
                    deployment_id,
                )
                .await?;
            results.push(test_run);
        }

        Ok(results)
    }

    /// Execute a single test (simulated for now)
    async fn execute_test(&self, function_name: &str) -> TestExecutionResult {
        // Run benchmark
        let runner = BenchmarkRunner::new(function_name.to_string(), 30);
        let (results, stats) = runner.run();

        // Simulate output (in production, this would be actual contract invocation result)
        let output = serde_json::json!({
            "function": function_name,
            "result": "success",
            "value": 42,
            "timestamp": Utc::now().to_rfc3339()
        });

        let output_hash = Self::hash_output(&output);

        TestExecutionResult {
            execution_time_ms: Some(stats.avg_ms),
            memory_bytes: results.first().and_then(|r| r.memory_bytes),
            cpu_instructions: results.first().and_then(|r| r.cpu_instructions),
            output,
            output_hash,
            error_message: None,
        }
    }

    /// Compare test result with baseline
    fn compare_with_baseline(
        &self,
        result: &TestExecutionResult,
        baseline: &TestBaseline,
    ) -> (
        TestStatus,
        bool,
        RegressionSeverity,
        Option<f64>,
        Option<bool>,
    ) {
        let mut regression_detected = false;
        let mut severity = RegressionSeverity::None;
        let mut degradation_percent = None;

        // Check output match
        let output_matches = result.output_hash == baseline.output_hash;

        // Check performance degradation
        if let Some(exec_time) = result.execution_time_ms {
            let baseline_time = baseline.baseline_execution_time_ms;
            let degradation = ((exec_time - baseline_time) / baseline_time) * 100.0;

            if degradation > self.thresholds.performance_degradation_minor {
                regression_detected = true;
                degradation_percent = Some(degradation);

                severity = if degradation > self.thresholds.performance_degradation_critical {
                    RegressionSeverity::Critical
                } else if degradation > self.thresholds.performance_degradation_major {
                    RegressionSeverity::Major
                } else {
                    RegressionSeverity::Minor
                };
            }
        }

        // Output mismatch is a major regression
        if !output_matches {
            regression_detected = true;
            if matches!(severity, RegressionSeverity::None | RegressionSeverity::Minor) {
                severity = RegressionSeverity::Major;
            }
        }

        let status = if regression_detected {
            TestStatus::Failed
        } else {
            TestStatus::Passed
        };

        (
            status,
            regression_detected,
            severity,
            degradation_percent,
            Some(output_matches),
        )
    }

    /// Hash output for comparison
    fn hash_output(output: &serde_json::Value) -> String {
        let serialized = serde_json::to_string(output).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get statistics for a contract
    pub async fn get_statistics(
        &self,
        contract_id: Uuid,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<RegressionStatistics, sqlx::Error> {
        // Calculate statistics
        sqlx::query("SELECT calculate_regression_statistics($1, $2, $3)")
            .bind(contract_id)
            .bind(period_start)
            .bind(period_end)
            .execute(&self.pool)
            .await?;

        // Fetch calculated statistics
        let stats: RegressionStatistics = sqlx::query_as(
            r#"SELECT 
                contract_id, period_start, period_end,
                total_runs, passed_runs, failed_runs,
                regressions_detected, false_positives, true_positives,
                detection_accuracy_percent, false_positive_rate_percent,
                avg_execution_time_ms, avg_degradation_percent
            FROM regression_test_statistics
            WHERE contract_id = $1 AND period_start = $2 AND period_end = $3"#,
        )
        .bind(contract_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }
}

#[derive(Debug)]
struct TestExecutionResult {
    execution_time_ms: Option<f64>,
    memory_bytes: Option<i64>,
    cpu_instructions: Option<i64>,
    output: serde_json::Value,
    output_hash: String,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegressionStatistics {
    pub contract_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_runs: i32,
    pub passed_runs: i32,
    pub failed_runs: i32,
    pub regressions_detected: i32,
    pub false_positives: i32,
    pub true_positives: i32,
    pub detection_accuracy_percent: Option<f64>,
    pub false_positive_rate_percent: Option<f64>,
    pub avg_execution_time_ms: Option<f64>,
    pub avg_degradation_percent: Option<f64>,
}
