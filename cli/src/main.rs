mod commands;
mod export;
mod import;
mod manifest;

use anyhow::Result;
use clap::{Parser, Subcommand};

const CLI_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (rustc ",
    env!("RUSTC_VERSION"),
    ")"
);

#[derive(Parser)]
#[command(name = "soroban-registry")]
#[command(version = CLI_VERSION, long_version = CLI_VERSION)]
#[command(about = "CLI tool for the Soroban Contract Registry", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, env = "SOROBAN_REGISTRY_API_URL", default_value = "http://localhost:3001")]
    api_url: String,
}

#[derive(Subcommand)]
enum Commands {
    Search {
        query: String,
        #[arg(long)]
        network: Option<String>,
        #[arg(long)]
        verified_only: bool,
    },

    Info {
        contract_id: String,
    },

    Publish {
        #[arg(long)]
        contract_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, default_value = "testnet")]
        network: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long)]
        publisher: String,
    },

    List {
        #[arg(long, default_value = "10")]
        limit: usize,
        #[arg(long)]
        network: Option<String>,
    },

    Export {
        id: String,
        #[arg(long, default_value = "contract.tar.gz")]
        output: String,
        #[arg(long, default_value = ".")]
        contract_dir: String,
    },

    Import {
        archive: String,
        #[arg(long, default_value = "testnet")]
        network: String,
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search { query, network, verified_only } => {
            commands::search(&cli.api_url, &query, network.as_deref(), verified_only).await?;
        }
        Commands::Info { contract_id } => {
            commands::info(&cli.api_url, &contract_id).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
            network,
            category,
            tags,
            publisher,
        } => {
            let tags_vec = tags
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            commands::publish(
                &cli.api_url,
                &contract_id,
                &name,
                description.as_deref(),
                &network,
                category.as_deref(),
                tags_vec,
                &publisher,
            )
            .await?;
        }
        Commands::List { limit, network } => {
            commands::list(&cli.api_url, limit, network.as_deref()).await?;
        }
        Commands::Export { id, output, contract_dir } => {
            commands::export(&cli.api_url, &id, &output, &contract_dir).await?;
        }
        Commands::Import { archive, network, output_dir } => {
            commands::import(&cli.api_url, &archive, &network, &output_dir).await?;
        }
    }

    Ok(())
}

