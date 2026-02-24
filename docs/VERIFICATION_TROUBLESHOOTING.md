# Verification Troubleshooting Guide

## Overview

This guide helps you diagnose and resolve common contract verification failures. For each issue, we provide symptoms, root causes, and step-by-step solutions.

## Quick Diagnostic Checklist

Before diving into specific errors, run through this checklist:

- [ ] Contract is deployed and exists on the specified network
- [ ] Using the exact soroban-sdk version that was used for deployment
- [ ] Source code is complete (no missing files or dependencies)
- [ ] `Cargo.toml` and `Cargo.lock` are included
- [ ] File encoding is UTF-8 (no special characters issues)
- [ ] Optimization level matches deployment settings
- [ ] No timestamps or non-deterministic code in source

## Common Verification Failures

### 1. Bytecode Mismatch

**Error:**
```json
{
  "error": "BYTECODE_MISMATCH",
  "message": "Compiled bytecode hash does not match on-chain hash",
  "expected_hash": "a3f2b8c9d1e4f5a6b7c8d9e0...",
  "actual_hash": "9f1a2b3c4d5e6f7a8b9c0d1e..."
}
```

**Symptoms:**
- Compilation succeeds, but hashes don't match
- "Verification failed" status

**Root Causes:**

#### Cause 1A: Wrong Compiler Version

The most common issue — you're using a different soroban-sdk version than deployment.

**Diagnosis:**
```bash
# Check what version was actually deployed
soroban contract inspect --id CDLZFC3... --network mainnet

# Compare with your Cargo.toml
grep soroban-sdk Cargo.toml
```

**Solution:**
```toml
# Cargo.toml - Use EXACT version (with = not ^)
[dependencies]
soroban-sdk = "=21.0.0"  # Pin to exact version
```

Then rebuild and retry verification.

#### Cause 1B: Wrong Optimization Level

Rust optimization flags affect output bytecode.

**Common optimization levels:**
- `opt-level = 0` — No optimization (debug)
- `opt-level = 1` — Basic optimization
- `opt-level = 2` — Full optimization
- `opt-level = 3` — Aggressive optimization
- `opt-level = "s"` — Optimize for size
- `opt-level = "z"` — Aggressively optimize for size (most common for Soroban)

**Diagnosis:**
```bash
# Try different optimization levels
for opt in 0 1 2 3 s z; do
  echo "Testing opt-level=$opt"
  cargo build --release --target wasm32-unknown-unknown
  sha256sum target/wasm32-unknown-unknown/release/*.wasm
done
```

**Solution:**
```toml
# Cargo.toml - Match deployment optimization
[profile.release]
opt-level = "z"  # Most Soroban contracts use "z"
```

#### Cause 1C: Dependency Version Mismatch

Different versions of dependencies produce different bytecode.

**Diagnosis:**
```bash
# Check if Cargo.lock is present
ls -la Cargo.lock

# Compare dependency versions
cargo tree | grep -v '(*)' | head -20
```

**Solution:**
1. Obtain the original `Cargo.lock` file from deployment
2. Commit `Cargo.lock` to source control
3. Use exact versions in `Cargo.toml`:

```toml
[dependencies]
soroban-sdk = "=21.0.0"
some-crate = "=1.2.3"  # Not "^1.2.3" or "~1.2.3"
```

#### Cause 1D: Build Environment Differences

Different operating systems, architectures, or toolchain versions can affect output.

**Diagnosis:**
```bash
# Check Rust version
rustc --version

# Check target
rustup target list --installed | grep wasm32
```

**Solution:**
Use Docker for reproducible builds:

```dockerfile
FROM rust:1.75-slim

RUN rustup target add wasm32-unknown-unknown
RUN cargo install soroban-cli --version 21.0.0

WORKDIR /build
COPY . .

RUN cargo build --release --target wasm32-unknown-unknown
```

Build with:
```bash
docker build -t contract-build .
docker run --rm -v $(pwd)/target:/build/target contract-build
```

#### Cause 1E: Non-Deterministic Code

Code that produces different output each time (timestamps, random numbers).

**Bad Example:**
```rust
use std::time::SystemTime;

pub fn get_version() -> u64 {
    // DON'T: Embeds build timestamp
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
}
```

**Solution:**
Use environment variables or const values:

```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}
```

---

### 2. Compilation Failed

**Error:**
```json
{
  "error": "COMPILATION_FAILED",
  "message": "Source code failed to compile",
  "details": "error[E0425]: cannot find function `transfer` in this scope"
}
```

**Symptoms:**
- Verification fails early in process
- Compilation errors in response

**Root Causes:**

#### Cause 2A: Missing Dependencies

**Diagnosis:**
```bash
# Try building locally
cargo build --release --target wasm32-unknown-unknown
```

**Solution:**
Ensure all dependencies are in `Cargo.toml`:

```toml
[dependencies]
soroban-sdk = "21.0.0"
# Add any missing crates
serde = { version = "1.0", features = ["derive"] }
```

#### Cause 2B: Syntax Errors

**Diagnosis:**
```bash
cargo check
```

**Solution:**
Fix syntax errors reported by compiler.

#### Cause 2C: Wrong Rust Edition

**Diagnosis:**
```bash
grep edition Cargo.toml
```

**Solution:**
```toml
[package]
edition = "2021"  # Soroban contracts use 2021 edition
```

#### Cause 2D: Missing Feature Flags

**Diagnosis:**
Check if contract uses features that need to be enabled.

**Solution:**
```toml
[features]
default = ["soroban-sdk/testutils"]

[dependencies]
soroban-sdk = { version = "21.0.0", features = ["alloc"] }
```

---

### 3. Contract Not Found

**Error:**
```json
{
  "error": "CONTRACT_NOT_FOUND",
  "message": "Contract does not exist on specified network",
  "contract_id": "CDLZFC3...",
  "network": "mainnet"
}
```

**Symptoms:**
- Verification fails immediately
- Contract ID doesn't exist on-chain

**Root Causes:**

#### Cause 3A: Wrong Network

You're verifying on mainnet but contract is on testnet (or vice versa).

**Diagnosis:**
```bash
# Check mainnet
soroban contract inspect --id CDLZFC3... --network mainnet

# Check testnet
soroban contract inspect --id CDLZFC3... --network testnet
```

**Solution:**
Specify correct network:

```bash
soroban-registry verify \
  --contract-id CDLZFC3... \
  --source ./src \
  --network testnet  # Correct network
```

#### Cause 3B: Typo in Contract ID

**Diagnosis:**
Double-check contract ID character-by-character.

**Solution:**
Copy contract ID directly from deployment output or blockchain explorer.

#### Cause 3C: Contract Not Yet Deployed

**Diagnosis:**
Check deployment status.

**Solution:**
Deploy contract first, then verify:

```bash
# Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/my_contract.wasm \
  --network mainnet \
  --source deployer

# Then verify
soroban-registry verify --contract-id <new_id> --source ./src
```

---

### 4. Invalid Source Format

**Error:**
```json
{
  "error": "INVALID_SOURCE_FORMAT",
  "message": "Source code encoding or format is invalid",
  "details": "Failed to decode base64 source code"
}
```

**Symptoms:**
- Source upload fails
- Encoding errors

**Root Causes:**

#### Cause 4A: File Encoding Issues

Non-UTF-8 characters in source files.

**Diagnosis:**
```bash
# Check file encoding
file -i src/**/*.rs

# Look for non-UTF-8 files
find src -name "*.rs" -exec file -i {} \; | grep -v utf-8
```

**Solution:**
Convert files to UTF-8:

```bash
# On macOS/Linux
iconv -f ISO-8859-1 -t UTF-8 src/lib.rs > src/lib.rs.utf8
mv src/lib.rs.utf8 src/lib.rs
```

#### Cause 4B: Incorrect Base64 Encoding

**Diagnosis:**
```bash
# Test base64 encoding/decoding
base64 -i src/lib.rs | base64 -d > /dev/null && echo "OK" || echo "FAIL"
```

**Solution:**
Use proper base64 encoding:

```bash
# Correct encoding
tar czf source.tar.gz src/ Cargo.toml Cargo.lock
base64 -i source.tar.gz > source.b64
```

#### Cause 4C: Missing Required Files

Verification needs `Cargo.toml`, `Cargo.lock`, and source files.

**Diagnosis:**
```bash
# Check archive contents
tar tzf source.tar.gz
```

**Solution:**
Ensure archive includes:
```
Cargo.toml
Cargo.lock
src/
  lib.rs
  (other .rs files)
```

---

### 5. ABI Mismatch

**Error:**
```json
{
  "error": "ABI_MISMATCH",
  "message": "Contract ABI does not match declared interface",
  "details": "Function 'transfer' signature mismatch: expected (Address, Address, i128), found (Address, i128)"
}
```

**Symptoms:**
- Bytecode matches, but interface validation fails
- Function signatures don't align

**Root Causes:**

#### Cause 5A: Wrong Source Version

You're verifying source for a different version of the contract.

**Diagnosis:**
Check git history:

```bash
git log --oneline --graph src/lib.rs
```

**Solution:**
Get source from the exact commit used for deployment:

```bash
git checkout <deployment-commit-hash>
```

#### Cause 5B: Conditional Compilation

Features or cfg flags change the interface.

**Diagnosis:**
```rust
// Example problem:
#[cfg(feature = "extra-functions")]
pub fn special_transfer() { ... }
```

**Solution:**
Use same features as deployment:

```bash
cargo build --release --features "extra-functions" --target wasm32-unknown-unknown
```

---

### 6. Build Timeout

**Error:**
```json
{
  "error": "BUILD_TIMEOUT",
  "message": "Contract compilation exceeded maximum time limit (300 seconds)"
}
```

**Symptoms:**
- Verification times out
- Complex contracts with many dependencies

**Root Causes:**

#### Cause 6A: Too Many Dependencies

Large dependency tree slows compilation.

**Diagnosis:**
```bash
cargo tree --depth 3
```

**Solution:**
- Remove unused dependencies
- Use `default-features = false` where possible:

```toml
[dependencies]
heavy-crate = { version = "1.0", default-features = false, features = ["minimal"] }
```

#### Cause 6B: Procedural Macros

Heavy macro expansion can slow builds.

**Solution:**
- Minimize macro usage
- Split into smaller crates
- Contact support for timeout increase (enterprise)

---

## Debugging Strategies

### Strategy 1: Local Compilation Test

Always test compilation locally before verification:

```bash
# Clean build
cargo clean
cargo build --release --target wasm32-unknown-unknown

# Check output
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Get hash
sha256sum target/wasm32-unknown-unknown/release/*.wasm
```

### Strategy 2: Binary Comparison

Compare your local build with on-chain bytecode:

```bash
# Fetch on-chain contract
soroban contract fetch \
  --id CDLZFC3... \
  --network mainnet \
  --out-file deployed.wasm

# Build locally
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/my_contract.wasm local.wasm

# Compare hashes
sha256sum deployed.wasm local.wasm

# If different, check binary diff
wasm-objdump -d deployed.wasm > deployed.wast
wasm-objdump -d local.wasm > local.wast
diff deployed.wast local.wast
```

### Strategy 3: Bisect Compiler Versions

If unsure which compiler version was used:

```bash
#!/bin/bash
# Try multiple versions
for version in 20.0.0 20.5.0 21.0.0 21.2.0; do
  echo "Testing soroban-sdk $version"

  # Update Cargo.toml
  sed -i "s/soroban-sdk = .*/soroban-sdk = \"=$version\"/" Cargo.toml

  # Build
  cargo clean
  cargo build --release --target wasm32-unknown-unknown 2>/dev/null

  # Check hash
  sha256sum target/wasm32-unknown-unknown/release/*.wasm
done
```

### Strategy 4: Check Verification Logs

Review detailed verification logs:

```bash
# Get verification details
curl -s https://registry.soroban.example/api/verifications/ver_abc123 | jq

# Look for specific errors
curl -s https://registry.soroban.example/api/verifications/ver_abc123/logs
```

### Strategy 5: Compare Working Example

Find a similar contract that verified successfully:

```bash
# Search for verified contracts
soroban-registry search "token" --verified-only

# Download source
soroban-registry source CDLZFC3... --output reference-src/

# Compare build configurations
diff reference-src/Cargo.toml mycontract/Cargo.toml
```

---

## Advanced Troubleshooting

### Reproducible Build Environment

Create a fully reproducible build environment:

**Dockerfile:**
```dockerfile
FROM rust:1.75.0-slim-bookworm

# Exact Rust version
RUN rustup default 1.75.0
RUN rustup target add wasm32-unknown-unknown

# Exact soroban-cli version
RUN cargo install soroban-cli --version 21.0.0 --locked

# Set build flags
ENV RUSTFLAGS="-C opt-level=z"
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=z

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

RUN cargo build --release --target wasm32-unknown-unknown --locked

CMD ["sha256sum", "target/wasm32-unknown-unknown/release/*.wasm"]
```

### Debugging Non-Determinism

Find sources of non-determinism:

```bash
# Build multiple times and compare
for i in {1..5}; do
  cargo clean
  cargo build --release --target wasm32-unknown-unknown
  sha256sum target/wasm32-unknown-unknown/release/*.wasm >> hashes.txt
done

# Check if all hashes are identical
sort hashes.txt | uniq | wc -l
# Should output: 1
```

If hashes differ, likely causes:
- Timestamps in build
- Random number generation
- Filesystem ordering
- Parallel compilation race conditions

### Inspect WASM Binary

Use `wasm-objdump` to inspect bytecode:

```bash
# Install wabt tools
brew install wabt  # macOS
# or: apt install wabt  # Linux

# Disassemble
wasm-objdump -d contract.wasm > contract.wast

# Check custom sections (may contain metadata)
wasm-objdump -s contract.wasm

# Look for differences
diff <(wasm-objdump -d deployed.wasm) <(wasm-objdump -d local.wasm)
```

---

## When to Report a Bug

If you've exhausted troubleshooting and believe it's a verification system bug, report with:

1. **Contract ID** and network
2. **Source code** (or link to public repo + commit hash)
3. **Exact build command** used
4. **Local hash vs expected hash**
5. **Verification error response**
6. **Logs from local build**

File at: https://github.com/ALIPHATICHYD/Soroban-Registry/issues

Tag with: `verification`, `bug`

---

## Success Checklist

Before submitting for verification:

- [ ] Contract deployed and confirmed on-chain
- [ ] Source builds locally without errors
- [ ] Using exact soroban-sdk version from deployment
- [ ] `Cargo.lock` committed and included
- [ ] All dependencies pinned to exact versions
- [ ] Optimization level matches deployment
- [ ] No non-deterministic code (timestamps, random)
- [ ] File encoding is UTF-8
- [ ] Archive includes `Cargo.toml`, `Cargo.lock`, `src/`

---

## Related Documentation

- [Verification Workflow](./VERIFICATION_WORKFLOW.md) - Complete verification process
- [Error Codes Reference](./ERROR_CODES.md) - All error codes explained
- [Security Best Practices](./SECURITY.md) - Contract security guidelines

## Support

For verification help:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Community Forum: https://community.stellar.org
- Tag with: `verification`, `troubleshooting`
