use std::env;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    PathBuf::from(
        env::var("CARGO_BIN_EXE_soroban-registry").expect("CARGO_BIN_EXE_soroban-registry not set"),
    )
}

#[test]
fn template_list_help() {
    let out = Command::new(binary())
        .args(["template", "list", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("list"), "expected 'list' in help output");
}

#[test]
fn template_clone_help() {
    let out = Command::new(binary())
        .args(["template", "clone", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("clone"), "expected 'clone' in help output");
}

#[test]
fn template_list_fails_gracefully_without_api() {
    let out = Command::new(binary())
        .args(["--api-url", "http://127.0.0.1:19999", "template", "list"])
        .output()
        .expect("failed to run binary");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("Invalid network"),
        "should not be a network parse error"
    );
}

#[test]
fn template_clone_fails_gracefully_without_api() {
    let out = Command::new(binary())
        .args([
            "--api-url",
            "http://127.0.0.1:19999",
            "template",
            "clone",
            "token",
            "my-token",
        ])
        .output()
        .expect("failed to run binary");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "arg parsing should succeed"
    );
}
