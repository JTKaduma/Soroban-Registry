use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

const DEFAULT_API_BASE: &str = "http://localhost:3001";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
    Auto, // Issue #78: Added Auto routing variant
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
            Network::Auto => write!(f, "auto"), // Issue #78
        }
    }
}

impl FromStr for Network {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "futurenet" => Ok(Network::Futurenet),
            _ => anyhow::bail!(
                "Invalid network: {}. Allowed values: mainnet, testnet, futurenet",
                s
            ),
            "auto" => Ok(Network::Auto), // Issue #78: Allow "auto" string
            _ => anyhow::bail!("Invalid network: {}. Allowed values: mainnet, testnet, futurenet, auto", s),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ConfigFile {
    defaults: Option<DefaultsSection>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct DefaultsSection {
    network: Option<String>,
    api_base: Option<String>,
    timeout: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub network: Network,
    pub api_base: String,
    pub timeout: u64,
}

pub fn resolve_runtime_config(
    cli_network: Option<String>,
    cli_api_base: Option<String>,
    cli_timeout: Option<u64>,
) -> Result<RuntimeConfig> {
    let config = load_defaults_section()?;

    let network = match cli_network.or(config.network) {
        Some(value) => value.parse::<Network>()?,
        None => Network::Testnet,
    };

    let api_base = cli_api_base
        .or(config.api_base)
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

    let timeout = cli_timeout
        .or(config.timeout)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);

    Ok(RuntimeConfig {
        network,
        api_base,
        timeout,
    })
}

pub fn show_config() -> Result<()> {
    let path = config_file_path().context("Could not determine home directory")?;
    let defaults = load_defaults_section()?;

    println!("Config file: {}", path.display());
    println!(
        "defaults.network = {}",
        defaults.network.unwrap_or_else(|| "testnet".to_string())
    );
    println!(
        "defaults.api_base = {}",
        defaults
            .api_base
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string())
    );
    println!(
        "defaults.timeout = {}",
        defaults.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)
    );

    Ok(())
}

pub fn edit_config() -> Result<()> {
    let path = config_file_path().context("Could not determine home directory")?;
    ensure_config_file_exists(&path)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to launch editor `{}`", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    // 2. Config File
    if let Some(config_path) = config_file_path() {
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

            let config: ConfigFile =
                toml::from_str(&content).with_context(|| "Failed to parse config file")?;

            if let Some(net_str) = config.network {
                return net_str.parse::<Network>();
            }
        }
    }

    // 3. Default
    Ok(Network::Mainnet)
}

fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|mut p| {
        p.push(".soroban-registry");
        p.push("config.toml");
        p
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_network_parsing() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("futurenet".parse::<Network>().unwrap(), Network::Futurenet);
        assert_eq!("auto".parse::<Network>().unwrap(), Network::Auto); // Issue #78
        assert_eq!("Mainnet".parse::<Network>().unwrap(), Network::Mainnet); // Case insensitive
        assert!("invalid".parse::<Network>().is_err());
    }

    #[test]
    fn test_load_config_file_with_defaults_section() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"[defaults]
network = "mainnet"
api_base = "http://localhost:9000"
timeout = 55
"#,
        )
        .unwrap();

        let parsed = load_config_file(&config_path).unwrap();
        let defaults = parsed.defaults.unwrap();

        assert_eq!(defaults.network.as_deref(), Some("mainnet"));
        assert_eq!(defaults.api_base.as_deref(), Some("http://localhost:9000"));
        assert_eq!(defaults.timeout, Some(55));
    }
}
        // Note: Integration tests involving file system would require mocking or temporary files.
    // Given the constraints and the environment, we focus on unit tests for parsing here.
    // `resolve_network` with file interaction is harder to test in isolation without dependency injection or mocking `dirs` / `fs`.
}
