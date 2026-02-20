mod commands;
mod config;
mod export;
mod import;
mod manifest;
mod multisig;
mod patch;
mod profiler;
mod test_framework;
mod wizard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use patch::Severity;

/// Soroban Registry CLI — discover, publish, verify, and deploy Soroban contracts
#[derive(Debug, Parser)]
#[command(name = "soroban-registry", version, about, long_about = None)]
pub struct Cli {
    /// Registry API URL
    #[arg(
        long,
        env = "SOROBAN_REGISTRY_API_URL",
        default_value = "http://localhost:3001"
    )]
    pub api_url: String,

    /// Stellar network to use (mainnet | testnet | futurenet)
    #[arg(long, global = true)]
    pub network: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search for contracts in the registry
    Search {
        /// Search query
        query: String,
        /// Only show verified contracts
        #[arg(long)]
        verified_only: bool,
    },

    /// Get detailed information about a contract
    Info {
        /// Contract ID to look up
        contract_id: String,
    },

    /// Publish a new contract to the registry
    Publish {
        /// On-chain contract ID
        #[arg(long)]
        contract_id: String,

        /// Human-readable contract name
        #[arg(long)]
        name: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,

        /// Contract category (e.g. token, defi, nft)
        #[arg(long)]
        category: Option<String>,

        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,

        /// Publisher Stellar address
        #[arg(long)]
        publisher: String,
    },

    /// List recent contracts
    List {
        /// Maximum number of contracts to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Migrate a contract to a new WASM
    Migrate {
        /// Contract ID to migrate
        #[arg(long)]
        contract_id: String,

        /// Path to the new WASM file
        #[arg(long)]
        wasm: String,

        /// Simulate a migration failure (for testing)
        #[arg(long)]
        simulate_fail: bool,

        /// Dry-run: show what would happen without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Export a contract archive (.tar.gz)
    Export {
        /// Contract registry ID (UUID)
        #[arg(long)]
        id: String,

        /// Output archive path
        #[arg(long, default_value = "contract-export.tar.gz")]
        output: String,

        /// Path to contract source directory
        #[arg(long, default_value = ".")]
        contract_dir: String,
    },

    /// Import a contract from an archive
    Import {
        /// Path to the archive file
        archive: String,

        /// Directory to extract into
        #[arg(long, default_value = "./imported")]
        output_dir: String,
    },

    /// Generate documentation from a contract WASM
    Doc {
        /// Path to contract WASM file
        contract_path: String,

        /// Output directory
        #[arg(long, default_value = "docs")]
        output: String,
    },

    /// Launch the interactive setup wizard
    Wizard {},

    /// Show command history
    History {
        /// Filter by search term
        #[arg(long)]
        search: Option<String>,

        /// Maximum number of entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Security patch management
    Patch {
        #[command(subcommand)]
        action: PatchCommands,
    },

    /// Multi-signature contract deployment workflow
    Multisig {
        #[command(subcommand)]
        action: MultisigCommands,
    },

    /// Profile contract execution performance
    Profile {
        /// Path to contract file
        contract_path: String,

        /// Method to profile
        #[arg(long)]
        method: Option<String>,

        /// Output JSON file
        #[arg(long)]
        output: Option<String>,

        /// Generate flame graph
        #[arg(long)]
        flamegraph: Option<String>,

        /// Compare with baseline profile
        #[arg(long)]
        compare: Option<String>,

        /// Show recommendations
        #[arg(long, default_value = "true")]
        recommendations: bool,
    },

    /// Run integration tests
    Test {
        /// Path to test file (YAML or JSON)
        test_file: String,

        /// Path to contract directory or file
        #[arg(long)]
        contract_path: Option<String>,

        /// Output JUnit XML report
        #[arg(long)]
        junit: Option<String>,

        /// Show coverage report
        #[arg(long, default_value = "true")]
        coverage: bool,

        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

/// Sub-commands for the `multisig` group
#[derive(Debug, Subcommand)]
pub enum MultisigCommands {
    /// Create a new multi-sig policy (defines signers and required threshold)
    CreatePolicy {
        /// Human-readable policy name
        #[arg(long)]
        name: String,

        /// Number of signatures required to approve (M-of-N)
        #[arg(long)]
        threshold: u32,

        /// Comma-separated list of authorized signer Stellar addresses
        #[arg(long)]
        signers: String,

        /// Seconds until proposals under this policy expire (default: 86400 = 24h)
        #[arg(long)]
        expiry_secs: Option<u32>,

        /// Stellar address of the policy creator
        #[arg(long)]
        created_by: String,
    },

    /// Create an unsigned deployment proposal
    CreateProposal {
        /// Human-readable name for the contract being proposed
        #[arg(long)]
        contract_name: String,

        /// On-chain contract ID (address)
        #[arg(long)]
        contract_id: String,

        /// WASM hash of the binary to deploy
        #[arg(long)]
        wasm_hash: String,

        /// Network (mainnet | testnet | futurenet)
        #[arg(long, default_value = "testnet")]
        network: String,

        /// UUID of the multi-sig policy to use
        #[arg(long)]
        policy_id: String,

        /// Stellar address of the proposer
        #[arg(long)]
        proposer: String,

        /// Optional description of the deployment
        #[arg(long)]
        description: Option<String>,
    },

    /// Sign a deployment proposal (add your approval)
    Sign {
        /// Proposal UUID to sign
        proposal_id: String,

        /// Your Stellar address
        #[arg(long)]
        signer: String,

        /// Optional hex-encoded signature payload for off-chain verification
        #[arg(long)]
        signature_data: Option<String>,
    },

    /// Execute an approved deployment proposal
    Execute {
        /// Proposal UUID to execute
        proposal_id: String,
    },

    /// Show full info for a proposal (signatures, policy, status)
    Info {
        /// Proposal UUID
        proposal_id: String,
    },

    /// List deployment proposals
    ListProposals {
        /// Filter by status (pending | approved | executed | expired | rejected)
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of proposals to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Clap sub-commands for the `patch` group (kept here so the match arm works)
// ─────────────────────────────────────────────────────────────────────────────

/// Sub-commands for the `patch` group
#[derive(Debug, Subcommand)]
pub enum PatchCommands {
    /// Create a new security patch
    Create {
        /// Target WASM version string
        #[arg(long)]
        version: String,
        /// New WASM hash
        #[arg(long)]
        hash: String,
        /// Severity level (critical|high|medium|low)
        #[arg(long)]
        severity: String,
        /// Rollout percentage (1-100)
        #[arg(long, default_value = "100")]
        rollout: u8,
    },
    /// Notify subscribers about a patch
    Notify {
        /// Patch UUID
        patch_id: String,
    },
    /// Apply a patch to a specific contract
    Apply {
        /// Contract ID to patch
        #[arg(long)]
        contract_id: String,
        /// Patch UUID to apply
        #[arg(long)]
        patch_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve network configuration
    let network = config::resolve_network(cli.network)?;

    match cli.command {
        // ── Existing commands ────────────────────────────────────────────────
        Commands::Search {
            query,
            verified_only,
        } => {
            commands::search(&cli.api_url, &query, network, verified_only).await?;
        }
        Commands::Info { contract_id } => {
            commands::info(&cli.api_url, &contract_id, network).await?;
        }
        Commands::Publish {
            contract_id,
            name,
            description,
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
                network,
                category.as_deref(),
                tags_vec,
                &publisher,
            )
            .await?;
        }
        Commands::List { limit } => {
            commands::list(&cli.api_url, limit, network).await?;
        }
        Commands::Migrate {
            contract_id,
            wasm,
            simulate_fail,
            dry_run,
        } => {
            commands::migrate(&cli.api_url, &contract_id, &wasm, simulate_fail, dry_run).await?;
        }
        Commands::Export {
            id,
            output,
            contract_dir,
        } => {
            commands::export(&cli.api_url, &id, &output, &contract_dir).await?;
        }
        Commands::Import {
            archive,
            output_dir,
        } => {
            commands::import(&cli.api_url, &archive, network, &output_dir).await?;
        }
        Commands::Doc {
            contract_path,
            output,
        } => {
            commands::doc(&contract_path, &output)?;
        }
        Commands::Wizard {} => {
            wizard::run(&cli.api_url).await?;
        }
        Commands::History { search, limit } => {
            wizard::show_history(search.as_deref(), limit)?;
        }
        Commands::Patch { action } => match action {
            PatchCommands::Create {
                version,
                hash,
                severity,
                rollout,
            } => {
                let sev = severity.parse::<Severity>()?;
                commands::patch_create(&cli.api_url, &version, &hash, sev, rollout).await?;
            }
            PatchCommands::Notify { patch_id } => {
                commands::patch_notify(&cli.api_url, &patch_id).await?;
            }
            PatchCommands::Apply {
                contract_id,
                patch_id,
            } => {
                commands::patch_apply(&cli.api_url, &contract_id, &patch_id).await?;
            }
        },

        // ── Multi-sig commands (issue #47) ───────────────────────────────────
        Commands::Multisig { action } => match action {
            MultisigCommands::CreatePolicy {
                name,
                threshold,
                signers,
                expiry_secs,
                created_by,
            } => {
                let signer_vec: Vec<String> =
                    signers.split(',').map(|s| s.trim().to_string()).collect();
                multisig::create_policy(
                    &cli.api_url,
                    &name,
                    threshold,
                    signer_vec,
                    expiry_secs,
                    &created_by,
                )
                .await?;
            }
            MultisigCommands::CreateProposal {
                contract_name,
                contract_id,
                wasm_hash,
                network: net_str,
                policy_id,
                proposer,
                description,
            } => {
                multisig::create_proposal(
                    &cli.api_url,
                    &contract_name,
                    &contract_id,
                    &wasm_hash,
                    &net_str,
                    &policy_id,
                    &proposer,
                    description.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Sign {
                proposal_id,
                signer,
                signature_data,
            } => {
                multisig::sign_proposal(
                    &cli.api_url,
                    &proposal_id,
                    &signer,
                    signature_data.as_deref(),
                )
                .await?;
            }
            MultisigCommands::Execute { proposal_id } => {
                multisig::execute_proposal(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::Info { proposal_id } => {
                multisig::proposal_info(&cli.api_url, &proposal_id).await?;
            }
            MultisigCommands::ListProposals { status, limit } => {
                multisig::list_proposals(&cli.api_url, status.as_deref(), limit).await?;
            }
        },
        Commands::Profile {
            contract_path,
            method,
            output,
            flamegraph,
            compare,
            recommendations,
        } => {
            commands::profile(
                &contract_path,
                method.as_deref(),
                output.as_deref(),
                flamegraph.as_deref(),
                compare.as_deref(),
                recommendations,
            )
            .await?;
        }
        Commands::Test {
            test_file,
            contract_path,
            junit,
            coverage,
            verbose,
        } => {
            commands::run_tests(
                &test_file,
                contract_path.as_deref(),
                junit.as_deref(),
                coverage,
                verbose,
            )
            .await?;
        }
    }

    Ok(())
}
