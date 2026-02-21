use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use serde_json::json;
use soroban_lint_core::{Analyzer, AutoFixer, Diagnostic, LintConfig, Severity};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "soroban-registry")]
#[command(about = "Smart contract linting tool for Soroban", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lint smart contracts
    Lint {
        /// Path to contract or directory
        #[arg(default_value = ".")]
        path: String,

        /// Minimum severity level to report
        #[arg(long, default_value = "warning")]
        level: String,

        /// Output format
        #[arg(long, default_value = "human")]
        format: String,

        /// Auto-apply safe fixes
        #[arg(long)]
        fix: bool,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,

        /// Comma-separated rules to run
        #[arg(long)]
        rules: Option<String>,

        /// Additional paths to ignore
        #[arg(long)]
        ignore: Option<String>,
    },

    /// List all available rules
    Rules {
        /// Output format
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Lint {
            path,
            level,
            format,
            fix,
            config,
            rules,
            ignore,
        } => {
            lint_command(path, level, format, fix, config, rules, ignore)?;
        }
        Commands::Rules { format } => {
            rules_command(format)?;
        }
    }

    Ok(())
}

fn lint_command(
    path: String,
    level: String,
    format: String,
    fix: bool,
    config_path: Option<String>,
    rules_filter: Option<String>,
    _ignore_filter: Option<String>,  // FIX: prefix with _ to suppress unused warning
) -> Result<()> {
    let start_time = Instant::now();

    // Load configuration
    let mut config = LintConfig::load(config_path.as_deref())?;

    // Override config with command-line arguments
    if level != "warning" {
        config.lint.level = level.clone();
    }

    let min_severity = config.min_severity();

    // Create analyzer
    let analyzer = Analyzer::new();

    // Parse filter rules if provided
    let rule_ids: Vec<&str> = if let Some(rules_str) = &rules_filter {
        rules_str.split(',').collect()
    } else {
        vec![]
    };

    // Collect all Rust files
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let path_obj = PathBuf::from(&path);

    if path_obj.is_file() {
        // Single file
        if path_obj.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&path)?;
            let file_diags = if rule_ids.is_empty() {
                analyzer.analyze_file(&path, &content)?
            } else {
                analyzer.analyze_file_with_rules(&path, &content, &rule_ids)?
            };
            diagnostics.extend(file_diags);
        }
    } else if path_obj.is_dir() {
        // Directory - recursively find all .rs files
        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let file_path = entry.path();
            let file_path_str = file_path.to_string_lossy().to_string();

            // Check if should be ignored
            if config.should_ignore(&file_path_str) {
                continue;
            }

            let content = fs::read_to_string(&file_path)?;
            let file_diags = if rule_ids.is_empty() {
                analyzer.analyze_file(&file_path_str, &content)?
            } else {
                analyzer.analyze_file_with_rules(&file_path_str, &content, &rule_ids)?
            };
            diagnostics.extend(file_diags);
        }
    }

    // Apply fixes if requested
    if fix {
        match AutoFixer::apply_fixes(&diagnostics) {
            Ok(count) => {
                if count > 0 {
                    println!("✅ Applied {} fixes", count);
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to apply fixes: {}", e);
            }
        }
    }

    // Filter by severity
    diagnostics = Analyzer::filter_by_severity(diagnostics, min_severity);

    // Sort diagnostics
    Analyzer::sort_diagnostics(&mut diagnostics);

    // Count by severity
    let error_count = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warning_count = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let info_count = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();

    let duration = start_time.elapsed();

    // Output results
    if format == "json" {
        output_json(&diagnostics, error_count, warning_count, info_count, duration)?;
    } else {
        output_human(&diagnostics, error_count, warning_count, info_count, duration);
    }

    // Exit code: 1 if errors/warnings found, 0 otherwise
    if error_count > 0 || (warning_count > 0 && min_severity <= Severity::Warning) {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}

fn rules_command(format: String) -> Result<()> {
    let analyzer = Analyzer::new();
    let rules = analyzer.list_rules();

    if format == "json" {
        let rules_json: Vec<_> = rules
            .iter()
            .map(|(id, severity)| {
                json!({
                    "id": id,
                    "severity": format!("{:?}", severity).to_lowercase()
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rules_json)?);
    } else {
        println!("Available Lint Rules:\n");
        // FIX: iterate over &rules so ownership is not moved into the for loop,
        // allowing rules.len() to be called afterwards
        for (id, severity) in &rules {
            let severity_str = format!("{:?}", severity).to_lowercase();
            println!("  {} [{}]", id, severity_str);
        }
        println!("\nTotal: {} rules", rules.len());
    }

    Ok(())
}

fn output_human(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    duration: std::time::Duration,
) {
    for diag in diagnostics {
        let severity_str = match diag.severity {
            Severity::Error => "[ERROR]".red().bold(),
            Severity::Warning => "[WARNING]".yellow().bold(),
            Severity::Info => "[INFO]".cyan(),
        };

        println!("{} {} {}", severity_str, diag.rule_id, diag.span);
        println!("  → {}", diag.message);

        if let Some(suggestion) = &diag.suggestion {
            println!("  Suggestion: {}", suggestion);
        }
        println!();
    }

    let summary = if error_count > 0 {
        format!(
            "Found {} {}, {} {}, {} {}",
            error_count,
            if error_count == 1 { "error" } else { "errors" },
            warning_count,
            if warning_count == 1 { "warning" } else { "warnings" },
            info_count,
            if info_count == 1 { "info" } else { "infos" }
        )
        .red()
        .bold()
    } else if warning_count > 0 {
        format!(
            "Found {} {}, {} {}",
            warning_count,
            if warning_count == 1 { "warning" } else { "warnings" },
            info_count,
            if info_count == 1 { "info" } else { "infos" }
        )
        .yellow()
    } else {
        "No issues found!".green().bold()
    };

    println!(
        "{}. Linting completed in {:.1}s.",
        summary,
        duration.as_secs_f64()
    );
}

fn output_json(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    duration: std::time::Duration,
) -> Result<()> {
    let output = json!({
        "summary": {
            "errors": error_count,
            "warnings": warning_count,
            "infos": info_count,
            "duration_ms": duration.as_millis()
        },
        "diagnostics": diagnostics
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}