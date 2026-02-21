/// JSON exporter for contract state
use crate::types::*;
use anyhow::Result;
use std::fs;

/// Exporter for state and diffs
pub struct StateExporter;

impl StateExporter {
    /// Export full contract state to JSON file
    pub fn export_to_file(state: &ContractState, output_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        fs::write(output_path, json)?;
        Ok(())
    }

    /// Export diff to JSON file
    pub fn export_diff_to_file(diff: &StateDiff, output_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(diff)?;
        fs::write(output_path, json)?;
        Ok(())
    }

    /// Print state as formatted JSON to stdout
    pub fn print_json(state: &ContractState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        println!("{}", json);
        Ok(())
    }

    /// Print diff as formatted JSON to stdout
    pub fn print_diff_json(diff: &StateDiff) -> Result<()> {
        let json = serde_json::to_string_pretty(diff)?;
        println!("{}", json);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exporter_creation() {
        let _exporter = StateExporter;
    }
}
