use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    // When running tests via cargo test, CARGO_BIN_EXE_<name> is set
    let name = "soroban-registry";
    let path = env::var(format!("CARGO_BIN_EXE_{}", name))
        .expect("Could not find binary path via env var");
    PathBuf::from(path)
}

#[test]
fn test_help_command() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("soroban-registry"));
    // Verify global flag is mentioned
    assert!(stdout.contains("--network"));
}

#[test]
fn test_invalid_network_flag() {
    let output = Command::new(get_binary_path())
        .arg("--network")
        .arg("invalid_value")
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The error message comes from `config::resolve_network` which calls `Network::from_str`
    // which returns "Invalid network: invalid_value. Allowed values: mainnet, testnet, futurenet"
    assert!(stderr.contains("Invalid network"));
}

#[test]
fn test_network_flag_global() {
    // This test ensures the flag is accepted globally even if command fails network call
    let output = Command::new(get_binary_path())
        .arg("--network")
        .arg("testnet")
        .arg("list")
        .arg("--limit")
        .arg("0") // minimize output/effect
        .output()
        .expect("Failed to execute command");

    // Even if it fails to connect to API, it should have parsed args successfully.
    // If API connection fails (likely in test env), output.status will be exit code 1 due to anyhow context "Failed to list contracts".
    // But stderr should NOT contain "Invalid network".
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Invalid network"));
    assert!(!stderr.contains("unexpected argument"));
}
