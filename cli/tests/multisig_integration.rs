use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let name_hyphen = "soroban-registry";
    let name_underscore = "soroban_registry";

    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name_underscore)) {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name_hyphen)) {
        return PathBuf::from(path);
    }

    // Fallback: look in target/debug relative to current dir
    let mut path = env::current_dir().expect("Failed to get current dir");
    path.push("target");
    path.push("debug");
    path.push(name_hyphen);
    if path.exists() {
        return path;
    }

    // Panic with clear message
    panic!("Could not find binary path via env var. Ensure `cargo build` has run.");
}

#[test]
fn test_multisig_help() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("create-policy"));
    assert!(stdout.contains("create-proposal"));
    assert!(stdout.contains("sign"));
    assert!(stdout.contains("execute"));
    assert!(stdout.contains("list-proposals"));
}

#[test]
fn test_create_policy_missing_args() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("create-policy")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required arguments were not provided"));
}

#[test]
fn test_create_proposal_help() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("create-proposal")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--contract-id"));
    assert!(stdout.contains("--wasm-hash"));
    assert!(stdout.contains("--policy-id"));
}
