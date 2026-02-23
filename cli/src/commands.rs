use crate::config::Network;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;
use shared::{extract_abi, generate_markdown};
use std::fs;
use std::path::Path;

use crate::patch::{PatchManager, Severity};
use crate::profiler;
use crate::test_framework;

pub async fn search(
    api_url: &str,
    query: &str,
    network: Network,
    verified_only: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/api/contracts?query={}&network={}",
        api_url, query, network
    );

    if verified_only {
        url.push_str("&verified_only=true");
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to search contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return Ok(());
    }

    for contract in items {
        let name = contract["name"].as_str().unwrap_or("Unknown");
        let contract_id = contract["contract_id"].as_str().unwrap_or("");
        let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
        let network = contract["network"].as_str().unwrap_or("");

        println!("\n{} {}", "●".green(), name.bold());
        println!("  ID: {}", contract_id.bright_black());
        println!(
            "  Status: {} | Network: {}",
            if is_verified {
                "✓ Verified".green()
            } else {
                "○ Unverified".yellow()
            },
            network.bright_blue()
        );

        if let Some(desc) = contract["description"].as_str() {
            println!("  {}", desc.bright_black());
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("Found {} contract(s)\n", items.len());

    Ok(())
}

pub async fn info(api_url: &str, contract_id: &str, network: Network) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts/{}?network={}",
        api_url, contract_id, network
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch contract info")?;
    if !response.status().is_success() {
        anyhow::bail!("Contract not found on {}", network);
    }

    let contract: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Information:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    println!(
        "\n{}: {}",
        "Name".bold(),
        contract["name"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{}: {}",
        "Contract ID".bold(),
        contract["contract_id"].as_str().unwrap_or("")
    );
    println!(
        "{}: {}",
        "Network".bold(),
        contract["network"].as_str().unwrap_or("").bright_blue()
    );

    let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
    println!(
        "{}: {}",
        "Verified".bold(),
        if is_verified {
            "✓ Yes".green()
        } else {
            "○ No".yellow()
        }
    );

    if let Some(desc) = contract["description"].as_str() {
        println!("\n{}: {}", "Description".bold(), desc);
    }

    if let Some(tags) = contract["tags"].as_array() {
        if !tags.is_empty() {
            print!("\n{}: ", "Tags".bold());
            for (i, tag) in tags.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}", tag.as_str().unwrap_or("").bright_magenta());
            }
            println!();
@@ -235,50 +235,61 @@ pub async fn list(api_url: &str, limit: usize, network: Network) -> Result<()> {
        let network = contract["network"].as_str().unwrap_or("");

        println!(
            "\n{}. {} {}",
            i + 1,
            name.bold(),
            if is_verified {
                "✓".green()
            } else {
                "".normal()
            }
        );
        println!(
            "   {} | {}",
            contract_id.bright_black(),
            network.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

fn extract_migration_id(migration: &serde_json::Value) -> Result<String> {
    let Some(migration_id) = migration["id"].as_str() else {
        eprintln!(
            "[error] migration response missing string id field: {}",
            migration
        );
        anyhow::bail!("Invalid migration response: missing id");
    };

    Ok(migration_id.to_string())
}

pub async fn migrate(
    api_url: &str,
    contract_id: &str,
    wasm_path: &str,
    simulate_fail: bool,
    dry_run: bool,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::fs;
    use tokio::process::Command;

    println!("\n{}", "Migration Tool".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // 1. Read WASM file
    let wasm_bytes = fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;

    // 2. Compute Hash
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = hex::encode(hasher.finalize());

    println!("Contract ID: {}", contract_id.green());
@@ -298,51 +309,51 @@ pub async fn migrate(

    // 3. Create Migration Record (Pending)
    let client = reqwest::Client::new();
    let create_url = format!("{}/api/migrations", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
    });

    print!("\nInitializing migration... ");
    let response = client
        .post(&create_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to contact registry API")?;

    if !response.status().is_success() {
        println!("{}", "Failed".red());
        let err = response.text().await?;
        anyhow::bail!("API Error: {}", err);
    }

    let migration: serde_json::Value = response.json().await?;
    let migration_id = extract_migration_id(&migration)?;
    println!("{}", "OK".green());
    println!("Migration ID: {}", migration_id);

    // 4. Execute Migration (Mock or Real)
    println!("\n{}", "Executing migration logic...".bold());

    // Check if soroban is installed
    let version_output = Command::new("soroban").arg("--version").output().await;

    let (status, log_output) = if version_output.is_err() {
        println!(
            "{}",
            "Warning: 'soroban' CLI not found. Running in MOCK mode.".yellow()
        );

        if simulate_fail {
            println!("{}", "Simulating FAILURE...".red());
            (
                shared::models::MigrationStatus::Failed,
                "Simulation: Migration failed as requested.".to_string(),
            )
        } else {
            println!("{}", "Simulating SUCCESS...".green());
            (
                shared::models::MigrationStatus::Success,
@@ -626,51 +637,54 @@ pub fn doc(contract_path: &str, output_dir: &str) -> Result<()> {
    println!("{} Documentation generated at {:?}", "✓".green(), out_path);
    Ok(())
}

pub async fn profile(
    contract_path: &str,
    method: Option<&str>,
    output: Option<&str>,
    flamegraph: Option<&str>,
    compare: Option<&str>,
    show_recommendations: bool,
) -> Result<()> {
    let path = Path::new(contract_path);
    if !path.exists() {
        anyhow::bail!("Contract file not found: {}", contract_path);
    }

    println!("\n{}", "Profiling contract...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let mut profiler = profiler::Profiler::new();
    profiler::simulate_execution(path, method, &mut profiler)?;
    let profile_data = profiler.finish(contract_path.to_string(), method.map(|s| s.to_string()));

    println!("\n{}", "Profile Results:".bold().green());
    println!(
        "Total Duration: {:.2}ms",
        profile_data.total_duration.as_secs_f64() * 1000.0
    );
    println!("Overhead: {:.2}%", profile_data.overhead_percent);
    println!("Functions Profiled: {}", profile_data.functions.len());

    let mut sorted_functions: Vec<_> = profile_data.functions.values().collect();
    sorted_functions.sort_by(|a, b| b.total_time.cmp(&a.total_time));

    println!("\n{}", "Top Functions:".bold());
    for (i, func) in sorted_functions.iter().take(10).enumerate() {
        println!(
            "{}. {} - {:.2}ms ({} calls, avg: {:.2}μs)",
            i + 1,
            func.name.bold(),
            func.total_time.as_secs_f64() * 1000.0,
            func.call_count,
            func.avg_time.as_secs_f64() * 1_000_000.0
        );
    }

    if let Some(output_path) = output {
        let json = serde_json::to_string_pretty(&profile_data)?;
        std::fs::write(output_path, json)
            .with_context(|| format!("Failed to write profile to: {}", output_path))?;
        println!("\n{} Profile exported to: {}", "✓".green(), output_path);
    }

@@ -687,202 +701,254 @@ pub async fn profile(
        let comparisons = profiler::compare_profiles(&baseline, &profile_data);

        println!("\n{}", "Comparison Results:".bold().yellow());
        for comp in comparisons.iter().take(10) {
            let sign = if comp.time_diff_ns > 0 { "+" } else { "" };
            println!(
                "{}: {} ({}{:.2}%, {:.2}ms → {:.2}ms)",
                comp.function.bold(),
                comp.status,
                sign,
                comp.time_diff_percent,
                comp.baseline_time.as_secs_f64() * 1000.0,
                comp.current_time.as_secs_f64() * 1000.0
            );
        }
    }

    if show_recommendations {
        let recommendations = profiler::generate_recommendations(&profile_data);
        println!("\n{}", "Recommendations:".bold().magenta());
        for (i, rec) in recommendations.iter().enumerate() {
            println!("{}. {}", i + 1, rec);
        }
    }

    Ok(())
}

pub async fn deps_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch contract dependencies")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Contract not found");
        }
        anyhow::bail!("Failed to fetch dependencies: {}", response.status());
    }

    let items: serde_json::Value = response.json().await?;
    let tree = items.as_array().context("Invalid response format")?;

    println!("\n{}", "Dependency Tree:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if tree.is_empty() {
        println!("{}", "No dependencies found.".yellow());
        return Ok(());
    }

    fn print_tree(nodes: &[serde_json::Value], prefix: &str, is_last: bool) {
        for (i, node) in nodes.iter().enumerate() {
            let name = node["name"].as_str().unwrap_or("Unknown");
            let constraint = node["constraint_to_parent"].as_str().unwrap_or("*");
            let contract_id = node["contract_id"].as_str().unwrap_or("");

            let is_node_last = i == nodes.len() - 1;
            let marker = if is_node_last {
                "└──"
            } else {
                "├──"
            };

            println!(
                "{}{} {} ({}) {}",
                prefix,
                marker.bright_black(),
                name.bold(),
                constraint.cyan(),
                if contract_id == "unknown" {
                    "[Unresolved]".red()
                } else {
                    "".normal()
                }
            );

            if let Some(children) = node["dependencies"].as_array() {
                if !children.is_empty() {
                    let new_prefix =
                        format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                    print_tree(children, &new_prefix, true);
                }
            }
        }
    }

    print_tree(tree, "", true);

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

pub async fn run_tests(
    test_file: &str,
    contract_path: Option<&str>,
    junit_output: Option<&str>,
    show_coverage: bool,
    verbose: bool,
) -> Result<()> {
    let test_path = Path::new(test_file);
    if !test_path.exists() {
        anyhow::bail!("Test file not found: {}", test_file);
    }

    let contract_dir = contract_path.unwrap_or(".");
    let mut runner = test_framework::TestRunner::new(contract_dir)?;

    println!("\n{}", "Running Integration Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let scenario = test_framework::load_test_scenario(test_path)?;

    if verbose {
        println!("\n{}: {}", "Scenario".bold(), scenario.name);
        if let Some(desc) = &scenario.description {
            println!("{}: {}", "Description".bold(), desc);
        }
        println!("{}: {}", "Steps".bold(), scenario.steps.len());
    }

    let start_time = std::time::Instant::now();
    let result = runner.run_scenario(scenario).await?;
    let total_time = start_time.elapsed();

    println!("\n{}", "Test Results:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    let status_icon = if result.passed { "✓" } else { "✗" };

    println!(
        "\n{} {} {} ({:.2}ms)",
        status_icon,
        "Scenario:".bold(),
        result.scenario.bold(),
        result.duration.as_secs_f64() * 1000.0
    );

    if !result.passed {
        if let Some(ref err) = result.error {
            println!("{} {}", "Error:".bold().red(), err);
        }
    }

    println!("\n{}", "Step Results:".bold());
    for (i, step) in result.steps.iter().enumerate() {
        let step_icon = if step.passed { "✓" } else { "✗" };

        println!(
            "  {}. {} {} ({:.2}ms)",
            i + 1,
            step_icon,
            step.step_name.bold(),
            step.duration.as_secs_f64() * 1000.0
        );

        if verbose {
            println!(
                "     Assertions: {}/{} passed",
                step.assertions_passed,
                step.assertions_passed + step.assertions_failed
            );
        }

        if let Some(ref err) = step.error {
            println!("     {}", err.red());
        }
    }

    if show_coverage {
        println!("\n{}", "Coverage Report:".bold().magenta());
        println!("  Contracts Tested: {}", result.coverage.contracts_tested);
        println!(
            "  Methods Tested: {}/{}",
            result.coverage.methods_tested, result.coverage.total_methods
        );
        println!("  Coverage: {:.2}%", result.coverage.coverage_percent);

        if result.coverage.coverage_percent < 80.0 {
            println!("  {} Low coverage detected!", "⚠".yellow());
        }
    }

    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result], Path::new(junit_path))?;
        println!(
            "\n{} JUnit XML report exported to: {}",
            "✓".green(),
            junit_path
        );
    }

    if total_time.as_secs() > 5 {
        println!(
            "\n{} Test execution took {:.2}s (target: <5s)",
            "⚠".yellow(),
            total_time.as_secs_f64()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    if !result.passed {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_migration_id;
    use serde_json::json;

    #[test]
    fn extract_migration_id_returns_id_for_valid_payload() {
        let payload = json!({"id": "migration-123"});
        let migration_id = extract_migration_id(&payload);
        assert!(migration_id.is_ok());
        assert_eq!(migration_id.unwrap_or_default(), "migration-123");
    }

    #[test]
    fn extract_migration_id_fails_when_missing_id() {
        let payload = json!({"status": "pending"});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }

    #[test]
    fn extract_migration_id_fails_when_id_is_not_string() {
        let payload = json!({"id": 99});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }
}