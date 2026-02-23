//! soroban-state-core — Contract state inspection library
//!
//! This library provides tools for inspecting, diffing, and debugging Soroban smart contract state.

pub mod client;
pub mod decoder;
pub mod differ;
pub mod dry_run;
pub mod exporter;
pub mod inspector;
pub mod types;

pub use client::StellarRpcClient;
pub use decoder::{decode_scval, decode_scval_bytes, decode_scval_native};
pub use differ::StateDiffer;
pub use dry_run::DryRunner;
pub use exporter::StateExporter;
pub use inspector::StateInspector;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modules_compile() {
        let _client = StellarRpcClient::testnet();
        let _inspector = StateInspector::testnet();
        let _differ = StateDiffer;
        let _runner = DryRunner::testnet();
    }
}
