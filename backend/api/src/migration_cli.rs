use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPool;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationCommand {
    Add { name: String },
    Rollback { steps: u32, yes: bool },
}

pub fn parse_command(args: &[String]) -> Result<Option<MigrationCommand>> {
    if args.first().map(String::as_str) != Some("migrate") {
        return Ok(None);
    }

    match args.get(1).map(String::as_str) {
        Some("add") => {
            let name = args
                .get(2)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("Usage: cargo run --bin api -- migrate add <name>"))?
                .to_owned();

            Ok(Some(MigrationCommand::Add { name }))
        }
        Some("rollback") => {
            let mut steps: u32 = 1;
            let mut yes = false;

            for arg in &args[2..] {
                if arg == "--yes" || arg == "-y" {
                    yes = true;
                    continue;
                }

                if let Some(raw_steps) = arg.strip_prefix("--steps=") {
                    steps = raw_steps
                        .parse::<u32>()
                        .context("--steps must be a positive integer")?;
                    continue;
                }

                return Err(anyhow!(
                    "Unknown argument: {arg}. Usage: cargo run --bin api -- migrate rollback --steps=<n> [--yes]"
                ));
            }

            if steps == 0 {
                return Err(anyhow!("--steps must be at least 1"));
            }

            Ok(Some(MigrationCommand::Rollback { steps, yes }))
        }
        _ => Err(anyhow!("Unknown migrate command. Supported: add, rollback")),
    }
}

pub fn migrations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../database/migrations")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/migrations"))
}

pub async fn execute(command: MigrationCommand, pool: &PgPool) -> Result<()> {
    match command {
        MigrationCommand::Add { name } => add_reversible_migration(&name),
        MigrationCommand::Rollback { steps, yes } => rollback_migrations(pool, steps, yes).await,
    }
}

fn add_reversible_migration(name: &str) -> Result<()> {
    let slug = slugify_name(name);
    if slug.is_empty() {
        return Err(anyhow!("Migration name must include letters or numbers"));
    }

    let version = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let migration_path = migrations_dir().join(format!("{version}_{slug}"));

    if migration_path.exists() {
        return Err(anyhow!(
            "Migration already exists: {}",
            migration_path.display()
        ));
    }

    fs::create_dir_all(&migration_path)
        .with_context(|| format!("Failed creating {}", migration_path.display()))?;

    let up_path = migration_path.join("up.sql");
    let down_path = migration_path.join("down.sql");

    fs::write(
        &up_path,
        format!(
            "-- Migration: {slug}\n-- Created: {}\n\nBEGIN;\n-- TODO: write forward migration SQL\n\nCOMMIT;\n",
            Utc::now().to_rfc3339()
        ),
    )
    .with_context(|| format!("Failed writing {}", up_path.display()))?;

    fs::write(
        &down_path,
        "-- Rollback for migration\n-- Prefer soft-deletion/archive patterns instead of hard deletes where possible.\n\nBEGIN;\n-- TODO: write rollback SQL\n\nCOMMIT;\n",
    )
    .with_context(|| format!("Failed writing {}", down_path.display()))?;

    println!(
        "Created reversible migration at {} (up.sql + down.sql)",
        migration_path.display()
    );

    Ok(())
}

async fn rollback_migrations(pool: &PgPool, steps: u32, yes: bool) -> Result<()> {
    ensure_migration_history_table(pool).await?;

    let applied = sqlx::query_as::<_, (i64, String)>(
        "SELECT version, description FROM _sqlx_migrations ORDER BY version DESC",
    )
    .fetch_all(pool)
    .await
    .context("Failed to inspect applied migrations")?;

    if applied.is_empty() {
        println!("No applied migrations found. Nothing to rollback.");
        return Ok(());
    }

    let to_rollback: Vec<(i64, String)> = applied.into_iter().take(steps as usize).collect();
    if to_rollback.is_empty() {
        println!("No migrations available to rollback for steps={steps}.");
        return Ok(());
    }

    let target_version = to_rollback
        .last()
        .map(|(version, _)| version - 1)
        .ok_or_else(|| anyhow!("Unable to determine rollback target version"))?;

    println!("The following migrations will be rolled back:");
    for (version, description) in &to_rollback {
        println!("  - {version}: {description}");
    }

    if !yes {
        print!(
            "Continue rollback of {} migration(s)? [y/N]: ",
            to_rollback.len()
        );
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read rollback confirmation")?;

        let confirmed = matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        if !confirmed {
            println!("Rollback cancelled.");
            return Ok(());
        }
    }

    let migrator = Migrator::new(migrations_dir())
        .await
        .context("Failed to load migrations from disk")?;

    let rollback_result = migrator.undo(pool, target_version).await;

    match rollback_result {
        Ok(()) => {
            for (version, description) in &to_rollback {
                sqlx::query(
                    "INSERT INTO migration_history(version, description, action, success, details) VALUES ($1, $2, 'rollback', true, 'Rollback completed')",
                )
                .bind(version)
                .bind(description)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to write rollback history for migration {version}"))?;
            }
        }
        Err(error) => {
            let error_message = error.to_string();
            for (version, description) in &to_rollback {
                let _ = sqlx::query(
                    "INSERT INTO migration_history(version, description, action, success, details) VALUES ($1, $2, 'rollback', false, $3)",
                )
                .bind(version)
                .bind(description)
                .bind(&error_message)
                .execute(pool)
                .await;
            }

            return Err(anyhow!("Rollback failed: {error_message}"));
        }
    }

    let remaining = sqlx::query_as::<_, (i64,)>(
        "SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("Failed verifying post-rollback migration state")?;

    let expected_latest = if target_version > 0 {
        Some(target_version)
    } else {
        None
    };

    let actual_latest = remaining.map(|tuple| tuple.0);
    if actual_latest != expected_latest {
        return Err(anyhow!(
            "Rollback verification failed. Expected latest version {:?}, got {:?}",
            expected_latest,
            actual_latest
        ));
    }

    println!(
        "Rollback successful. Reverted {} migration(s); latest version is {:?}.",
        to_rollback.len(),
        actual_latest
    );

    Ok(())
}

async fn ensure_migration_history_table(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS migration_history (
            id BIGSERIAL PRIMARY KEY,
            version BIGINT NOT NULL,
            description TEXT NOT NULL,
            action TEXT NOT NULL,
            success BOOLEAN NOT NULL,
            details TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .context("Failed to ensure migration_history table exists")?;

    Ok(())
}

fn slugify_name(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    slug.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::slugify_name;

    #[test]
    fn slugifies_names() {
        assert_eq!(slugify_name("Add Users Table"), "add_users_table");
        assert_eq!(slugify_name("___hello---world___"), "hello_world");
    }
}
