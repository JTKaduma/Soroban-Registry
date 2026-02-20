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
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

pub async fn publish(
    api_url: &str,
    contract_id: &str,
    name: &str,
    description: Option<&str>,
    network: Network,
    category: Option<&str>,
    tags: Vec<String>,
    publisher: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "name": name,
        "description": description,
        "network": network.to_string(),
        "category": category,
        "tags": tags,
        "publisher_address": publisher,
    });

    println!("\n{}", "Publishing contract...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to publish contract")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to publish: {}", error_text);
    }

    let contract: serde_json::Value = response.json().await?;

    println!("{}", "✓ Contract published successfully!".green().bold());
    println!(
        "\n{}: {}",
        "Name".bold(),
        contract["name"].as_str().unwrap_or("")
    );
    println!(
        "{}: {}",
        "ID".bold(),
        contract["contract_id"].as_str().unwrap_or("")
    );
    println!(
        "{}: {}",
        "Network".bold(),
        contract["network"].as_str().unwrap_or("").bright_blue()
    );
    println!();

    Ok(())
}

pub async fn list(api_url: &str, limit: usize, network: Network) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts?page_size={}&network={}",
        api_url, limit, network
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to list contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

    println!("\n{}", "Recent Contracts:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return Ok(());
    }

    for (i, contract) in items.iter().enumerate() {
        let name = contract["name"].as_str().unwrap_or("Unknown");
        let contract_id = contract["contract_id"].as_str().unwrap_or("");
        let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
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
    println!("WASM Path:   {}", wasm_path);
    println!("WASM Hash:   {}", wasm_hash.bright_black());
    println!("Size:        {} bytes", wasm_bytes.len());

    if dry_run {
        println!("\n{}", "[DRY RUN] No changes will be made.".yellow().bold());
        println!("Would create migration record...");
        println!(
            "Would execute: soroban contract invoke --id {} --wasm {} ...",
            contract_id, wasm_path
        );
        return Ok(());
    }

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
    let migration_id = migration["id"].as_str().unwrap();
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
                "Simulation: Migration succeeded.".to_string(),
            )
        }
    } else {
        // Real execution would go here. For now we will just mock it even if soroban exists
        // because we don't have a real contract to invoke in this environment.
        println!(
            "{}",
            "Soroban CLI found, but full integration is pending. Running in MOCK mode.".yellow()
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
                "Simulation: Migration executed successfully via soroban CLI (mocked).".to_string(),
            )
        }
    };

    // 5. Update Status
    let update_url = format!("{}/api/migrations/{}", api_url, migration_id);
    let update_payload = json!({
        "status": status,
        "log_output": log_output
    });

    let update_res = client
        .put(&update_url)
        .json(&update_payload)
        .send()
        .await
        .context("Failed to update migration status")?;

    if !update_res.status().is_success() {
        println!("{}", "Failed to update status!".red());
    } else {
        println!("\n{}", "Migration recorded successfully.".green().bold());
        if status == shared::models::MigrationStatus::Failed {
            println!("{}", "Status: FAILED".red().bold());
        } else {
            println!("{}", "Status: SUCCESS".green().bold());
        }
    }

    Ok(())
}

pub async fn export(
    api_url: &str,
    contract_id: &str,
    output: &str,
    contract_dir: &str,
) -> Result<()> {
    println!("\n{}", "Exporting contract...".bold().cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}", api_url, contract_id);

    let (name, network) = match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let data: serde_json::Value = resp.json().await?;
            (
                data["name"].as_str().unwrap_or(contract_id).to_string(),
                data["network"].as_str().unwrap_or("unknown").to_string(),
            )
        }
        _ => (contract_id.to_string(), "unknown".to_string()),
    };

    let source = std::path::Path::new(contract_dir);
    anyhow::ensure!(
        source.is_dir(),
        "contract directory does not exist: {}",
        contract_dir
    );

    crate::export::create_archive(
        source,
        std::path::Path::new(output),
        contract_id,
        &name,
        &network,
    )?;

    println!("{}", "✓ Export complete!".green().bold());
    println!("  {}: {}", "Output".bold(), output);
    println!("  {}: {}", "Contract".bold(), contract_id.bright_black());
    println!("  {}: {}\n", "Name".bold(), name);

    Ok(())
}

pub async fn import(
    api_url: &str,
    archive: &str,
    network: Network,
    output_dir: &str,
) -> Result<()> {
    println!("\n{}", "Importing contract...".bold().cyan());

    let archive_path = std::path::Path::new(archive);
    anyhow::ensure!(archive_path.is_file(), "archive not found: {}", archive);

    let dest = std::path::Path::new(output_dir);

    let manifest = crate::import::extract_and_verify(archive_path, dest)?;

    println!(
        "{}",
        "✓ Import complete — integrity verified!".green().bold()
    );
    println!(
        "  {}: {}",
        "Contract".bold(),
        manifest.contract_id.bright_black()
    );
    println!("  {}: {}", "Name".bold(), manifest.name);
    println!(
        "  {}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("  {}: {}", "SHA-256".bold(), manifest.sha256.bright_black());
    println!("  {}: {}", "Exported At".bold(), manifest.exported_at);
    println!(
        "  {}: {} file(s)",
        "Contents".bold(),
        manifest.contents.len()
    );
    println!("  {}: {}", "Extracted To".bold(), output_dir);

    println!(
        "\n  {} To register on {}, run:",
        "→".bright_black(),
        network.to_string().bright_blue()
    );
    println!(
        "    soroban-registry publish --contract-id {} --name \"{}\" --network {} --publisher <address>\n",
        manifest.contract_id, manifest.name, network
    );

    Ok(())
}

fn severity_colored(sev: &Severity) -> colored::ColoredString {
    match sev {
        Severity::Critical => "CRITICAL".red().bold(),
        Severity::High => "HIGH".yellow().bold(),
        Severity::Medium => "MEDIUM".cyan(),
        Severity::Low => "LOW".normal(),
    }
}

pub async fn patch_create(
    api_url: &str,
    version: &str,
    hash: &str,
    severity: Severity,
    rollout: u8,
) -> Result<()> {
    println!("\n{}", "Creating security patch...".bold().cyan());

    let patch = PatchManager::create(api_url, version, hash, severity, rollout).await?;

    println!("{}", "✓ Patch created!".green().bold());
    println!("  {}: {}", "ID".bold(), patch.id);
    println!("  {}: {}", "Target Version".bold(), patch.target_version);
    println!(
        "  {}: {}",
        "Severity".bold(),
        severity_colored(&patch.severity)
    );
    println!(
        "  {}: {}",
        "New WASM Hash".bold(),
        patch.new_wasm_hash.bright_black()
    );
    println!("  {}: {}%\n", "Rollout".bold(), patch.rollout_percentage);

    if matches!(patch.severity, Severity::Critical | Severity::High) {
        println!(
            "  {} {}",
            "⚠".red(),
            format!(
                "{} severity — immediate action recommended",
                severity_colored(&patch.severity)
            )
            .red()
        );
    }

    Ok(())
}

pub async fn patch_notify(api_url: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Identifying vulnerable contracts...".bold().cyan());

    let (patch, contracts) = PatchManager::find_vulnerable(api_url, patch_id).await?;

    println!(
        "\n{} {} patch for version {}",
        "⚠".bold(),
        severity_colored(&patch.severity),
        patch.target_version.bold()
    );
    println!("{}", "=".repeat(80).cyan());

    if contracts.is_empty() {
        println!("{}", "No vulnerable contracts found.".green());
        return Ok(());
    }

    for (i, c) in contracts.iter().enumerate() {
        let cid = c["contract_id"].as_str().unwrap_or("");
        let name = c["name"].as_str().unwrap_or("Unknown");
        let net = c["network"].as_str().unwrap_or("");
        println!(
            "  {}. {} ({}) [{}]",
            i + 1,
            name.bold(),
            cid.bright_black(),
            net.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("{} vulnerable contract(s) found\n", contracts.len());

    Ok(())
}

pub async fn patch_apply(api_url: &str, contract_id: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Applying security patch...".bold().cyan());

    let audit = PatchManager::apply(api_url, contract_id, patch_id).await?;

    println!("{}", "✓ Patch applied successfully!".green().bold());
    println!("  {}: {}", "Contract".bold(), audit.contract_id);
    println!("  {}: {}", "Patch".bold(), audit.patch_id);
    println!("  {}: {}\n", "Applied At".bold(), audit.applied_at);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// DOCUMENTATION GENERATION
// ─────────────────────────────────────────────────────────────────────────────

pub fn doc(contract_path: &str, output_dir: &str) -> Result<()> {
    println!("{}", "Generating documentation...".bold().cyan());

    // Extract ABI
    // Note: This requires the `shared` crate to export `extract_abi`
    let abi_entries = extract_abi(contract_path).context("Failed to extract ABI from WASM")?;

    let filename = std::path::Path::new(contract_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("contract");

    // Generate Markdown
    let markdown = generate_markdown(&abi_entries, filename);

    // Write to output
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    let out_path = std::path::Path::new(output_dir).join(format!("{}.md", filename));
    fs::write(&out_path, markdown)
        .with_context(|| format!("Failed to write documentation to {:?}", out_path))?;

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
    println!("Total Duration: {:.2}ms", profile_data.total_duration.as_secs_f64() * 1000.0);
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

    if let Some(flame_path) = flamegraph {
        profiler::generate_flame_graph(&profile_data, Path::new(flame_path))?;
        println!("{} Flame graph generated: {}", "✓".green(), flame_path);
    }

    if let Some(baseline_path) = compare {
        let baseline_json = std::fs::read_to_string(baseline_path)
            .with_context(|| format!("Failed to read baseline: {}", baseline_path))?;
        let baseline: profiler::ProfileData = serde_json::from_str(&baseline_json)?;

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
        println!("  Methods Tested: {}/{}", 
            result.coverage.methods_tested, 
            result.coverage.total_methods
        );
        println!("  Coverage: {:.2}%", result.coverage.coverage_percent);
        
        if result.coverage.coverage_percent < 80.0 {
            println!("  {} Low coverage detected!", "⚠".yellow());
        }
    }

    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result], Path::new(junit_path))?;
        println!("\n{} JUnit XML report exported to: {}", "✓".green(), junit_path);
    }

    if total_time.as_secs() > 5 {
        println!("\n{} Test execution took {:.2}s (target: <5s)", 
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
