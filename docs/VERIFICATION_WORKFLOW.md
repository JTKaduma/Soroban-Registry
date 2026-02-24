# Smart Contract Verification Workflow

## Overview

Contract verification proves that the source code you claim matches the bytecode deployed on the Stellar blockchain. This establishes trust by allowing users to audit the contract's behavior before interacting with it.

## Verification Process Flow

```
┌─────────────────┐
│ User Submits    │
│ Source Code +   │──┐
│ Contract ID     │  │
└─────────────────┘  │
                     │
                     ▼
┌─────────────────────────────────────┐
│ 1. Validation                       │
│  ✓ Contract ID exists on-chain      │
│  ✓ Source code format valid         │
│  ✓ No previous verification         │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 2. Compilation                      │
│  • Parse source code                │
│  • Set up build environment         │
│  • Compile with soroban-sdk         │
│  • Generate WASM bytecode           │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 3. Bytecode Comparison              │
│  • Hash compiled WASM               │
│  • Fetch on-chain bytecode hash     │
│  • Compare hashes                   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 4. ABI Validation (Optional)        │
│  • Extract contract interface       │
│  • Compare with declared ABI        │
│  • Validate method signatures       │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 5. Store Results                    │
│  • Save verification status         │
│  • Store source code                │
│  • Update contract metadata         │
│  • Emit verification event          │
└─────────────────────────────────────┘
               │
               ▼
        ┌──────┴──────┐
        │             │
    SUCCESS        FAILURE
        │             │
        ▼             ▼
   Badge on      Error Report
   Contract      with Details
```

## What Gets Verified

### 1. Bytecode Match

The core verification ensures that compiling your source code produces **exactly** the same WASM bytecode as what's deployed on-chain.

**Process:**
1. Compile source code → WASM bytecode
2. Hash bytecode using SHA-256
3. Fetch on-chain contract bytecode hash via Stellar RPC
4. Compare: `hash(compiled_wasm) == on_chain_hash`

**Success Criteria:** Hashes match exactly (byte-for-byte identical).

### 2. ABI Matching

Validates that the contract's interface matches expectations.

**Checks:**
- Function names and signatures
- Parameter types and names
- Return types
- Custom types and structs

**Success Criteria:** All public functions in source match the extracted ABI from bytecode.

### 3. Source Integrity

Ensures source code is complete and buildable.

**Checks:**
- Valid Rust syntax
- All dependencies specified in `Cargo.toml`
- No missing files or modules
- Proper soroban-sdk version

**Success Criteria:** Code compiles without errors.

## Verification Methods

### Method 1: Source Code Verification (Recommended)

Submit complete source code for compilation and comparison.

**API Endpoint:**
```http
POST /api/contracts/verify
Content-Type: application/json

{
  "contract_id": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "source_code": "base64_encoded_source",
  "compiler_version": "21.0.0",
  "optimization_level": "z"
}
```

**Response (Success):**
```json
{
  "status": "verified",
  "verification_id": "ver_abc123",
  "timestamp": "2026-02-24T12:34:56Z",
  "bytecode_hash": "a3f2b8c9d1e4...",
  "compiler_used": "soroban-sdk 21.0.0",
  "build_reproducible": true
}
```

**Response (Failure):**
```json
{
  "status": "failed",
  "verification_id": "ver_abc124",
  "timestamp": "2026-02-24T12:35:12Z",
  "error": {
    "code": "BYTECODE_MISMATCH",
    "message": "Compiled bytecode hash does not match on-chain hash",
    "expected_hash": "a3f2b8c9d1e4...",
    "actual_hash": "9f1a2b3c4d5e...",
    "details": "Possible causes: wrong compiler version, optimization settings, or source code"
  }
}
```

### Method 2: Binary Hash Verification

Verify by directly providing the WASM bytecode hash (for pre-compiled contracts).

**API Endpoint:**
```http
POST /api/contracts/verify-hash
Content-Type: application/json

{
  "contract_id": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "bytecode_hash": "a3f2b8c9d1e4..."
}
```

**Use case:** When source code cannot be disclosed, but you want to verify the bytecode hash is correct.

## Success Criteria and Failure Reasons

### Verification Success

A verification succeeds when **all** of the following are true:

1. ✅ Contract exists on-chain at specified ID
2. ✅ Source code compiles successfully
3. ✅ Compiled bytecode hash matches on-chain hash
4. ✅ ABI extracted from bytecode matches declared interface
5. ✅ No compiler warnings or errors

### Common Failure Reasons

| Error Code | Reason | How to Fix |
|------------|--------|------------|
| `CONTRACT_NOT_FOUND` | Contract ID doesn't exist on-chain | Verify contract ID, check network |
| `BYTECODE_MISMATCH` | Compiled bytecode ≠ on-chain bytecode | Check compiler version, optimization settings |
| `COMPILATION_FAILED` | Source code doesn't compile | Fix syntax errors, ensure dependencies correct |
| `ABI_MISMATCH` | Function signatures don't match | Ensure source matches deployed version |
| `INVALID_SOURCE_FORMAT` | Source code encoding/format issue | Check file encoding (UTF-8), proper base64 |
| `COMPILER_VERSION_MISMATCH` | Wrong soroban-sdk version used | Use exact version that was originally deployed |
| `OPTIMIZATION_MISMATCH` | Wrong optimization level | Try different `-C opt-level` settings |
| `MISSING_DEPENDENCIES` | Cargo.toml missing dependencies | Include complete dependency tree |
| `BUILD_TIMEOUT` | Compilation took too long | Simplify contract or contact support |

## Retry Logic and Backoff

If verification fails due to transient issues (RPC timeout, network errors), the system automatically retries:

**Retry Strategy:**
```
Attempt 1: Immediate
Attempt 2: +2 seconds
Attempt 3: +4 seconds
Attempt 4: +8 seconds
Attempt 5: +16 seconds (final)
```

**Retryable Errors:**
- RPC timeout
- Network connectivity issues
- Temporary service unavailability
- Rate limiting

**Non-Retryable Errors:**
- Bytecode mismatch
- Compilation failure
- Invalid source code
- Contract not found

## Using the CLI for Verification

### Basic Verification

```bash
soroban-registry verify \
  --contract-id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \
  --source ./src \
  --network mainnet
```

### Advanced Options

```bash
soroban-registry verify \
  --contract-id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC \
  --source ./src \
  --compiler-version 21.0.0 \
  --optimization-level z \
  --network mainnet \
  --wait-for-confirmation \
  --verbose
```

**Flags:**
- `--source`: Path to contract source directory
- `--compiler-version`: Specific soroban-sdk version
- `--optimization-level`: Rust optimization (`0`, `1`, `2`, `3`, `s`, `z`)
- `--wait-for-confirmation`: Wait for verification to complete
- `--verbose`: Show detailed compilation output

### Check Verification Status

```bash
soroban-registry info CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

Output:
```
Contract: CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
Status: Verified ✓
Verified at: 2026-02-24 12:34:56 UTC
Compiler: soroban-sdk 21.0.0
Source available: Yes
```

## Verification via Web Interface

### Step 1: Navigate to Contract

Go to: `https://registry.soroban.example/contracts/{contract_id}`

### Step 2: Click "Verify Contract"

### Step 3: Upload Source Code

**Option A - Upload ZIP:**
- Upload `.zip` file containing source code
- Must include `Cargo.toml` and `src/` directory
- Max size: 10 MB

**Option B - Paste Source:**
- Paste contract source code directly
- Specify `Cargo.toml` dependencies separately

**Option C - GitHub Integration:**
- Link to GitHub repository + commit hash
- Automatically fetches source code

### Step 4: Specify Build Settings

- **Compiler Version**: Select soroban-sdk version (e.g., `21.0.0`)
- **Optimization Level**: Select optimization (default: `z` for size)
- **Network**: Mainnet, Testnet, or Futurenet

### Step 5: Submit and Monitor

- Verification runs in background (typically 30-120 seconds)
- Real-time progress updates shown
- Email notification on completion

## Build Reproducibility

For verification to succeed, builds must be **reproducible** — compiling the same source code twice should produce identical bytecode.

### Ensuring Reproducibility

1. **Pin soroban-sdk version** in `Cargo.toml`:
   ```toml
   [dependencies]
   soroban-sdk = "=21.0.0"  # Use exact version
   ```

2. **Pin all dependencies:**
   ```toml
   [dependencies]
   some-crate = "=1.2.3"  # Not "^1.2" or "~1.2"
   ```

3. **Use `Cargo.lock`:**
   - Commit `Cargo.lock` to repository
   - Ensures exact dependency versions

4. **Specify target explicitly:**
   ```toml
   [build]
   target = "wasm32-unknown-unknown"
   ```

5. **Disable timestamps in build:**
   ```toml
   [profile.release]
   opt-level = "z"
   debug = false
   ```

### Checking Reproducibility Locally

```bash
# Build twice and compare
cargo build --release --target wasm32-unknown-unknown
mv target/wasm32-unknown-unknown/release/my_contract.wasm build1.wasm

cargo clean
cargo build --release --target wasm32-unknown-unknown
mv target/wasm32-unknown-unknown/release/my_contract.wasm build2.wasm

# Compare bytecode
sha256sum build1.wasm build2.wasm
# Hashes should be identical
```

## Verification Timeline

| Stage | Typical Duration | Max Duration |
|-------|------------------|--------------|
| Validation | < 1 second | 5 seconds |
| Compilation | 10-60 seconds | 300 seconds (5 min) |
| Bytecode Fetch | 1-5 seconds | 30 seconds |
| Hash Comparison | < 1 second | 1 second |
| Storage | 1-2 seconds | 10 seconds |
| **Total** | **15-90 seconds** | **5 minutes** |

**Note:** Complex contracts with many dependencies may take longer to compile.

## Security Considerations

### Source Code Privacy

- Verified source code is **publicly visible** in the registry
- Do not include secrets, API keys, or private information
- Consider using environment variables for configuration

### Malicious Source Code

- Verification does NOT guarantee contract safety
- Verified contracts can still be malicious
- Always audit source code before trusting a contract
- Check for common vulnerabilities (see [Security Best Practices](./SECURITY.md))

### Verification Spoofing

- Only trust verifications from the official Soroban Registry
- Check verification timestamp and verifier identity
- Be wary of "verified" claims without registry confirmation

## Verification Badge

Successfully verified contracts display a badge:

```
✓ Verified on 2026-02-24
Source code matches on-chain bytecode
```

The badge includes:
- Verification timestamp
- Compiler version used
- Link to view source code
- Hash of verified bytecode

## Automation and CI/CD

### GitHub Actions Example

```yaml
name: Verify Contract

on:
  push:
    tags:
      - 'v*'

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Soroban CLI
        run: |
          cargo install soroban-cli
          cargo install soroban-registry-cli

      - name: Build Contract
        run: |
          cd contracts/my-contract
          cargo build --release --target wasm32-unknown-unknown

      - name: Deploy to Mainnet
        run: |
          soroban contract deploy \
            --wasm target/wasm32-unknown-unknown/release/my_contract.wasm \
            --network mainnet \
            --source deployer
        env:
          SOROBAN_SECRET_KEY: ${{ secrets.DEPLOYER_SECRET }}

      - name: Verify Contract
        run: |
          soroban-registry verify \
            --contract-id ${{ env.CONTRACT_ID }} \
            --source ./src \
            --network mainnet \
            --wait-for-confirmation
```

## Metrics and Monitoring

Track verification health:

```promql
# Verification success rate
rate(soroban_verification_total{result="success"}[10m]) /
  rate(soroban_verification_total[10m])

# P99 verification latency
histogram_quantile(0.99,
  sum(rate(soroban_verification_latency_seconds_bucket[5m])) by (le))

# Queue depth
soroban_verification_queue_depth
```

## FAQs

### Q: How long does verification take?
**A:** Typically 15-90 seconds. Complex contracts may take up to 5 minutes.

### Q: Can I verify contracts deployed by others?
**A:** Yes! Anyone can submit source code for verification of any contract.

### Q: What if my source code contains proprietary logic?
**A:** Verified source is public. If you can't disclose source, use hash-based verification or don't verify.

### Q: Why does my verification fail with "bytecode mismatch"?
**A:** Most common causes are wrong compiler version or optimization settings. See [Troubleshooting Guide](./VERIFICATION_TROUBLESHOOTING.md).

### Q: Can I update verification after contract deployment?
**A:** No. Verification is tied to a specific on-chain bytecode. If you upgrade the contract, you must verify the new version separately.

### Q: Do I need to verify on each network separately?
**A:** Yes. Mainnet, Testnet, and Futurenet are separate, so verify on each network where your contract is deployed.

### Q: What compiler versions are supported?
**A:** All stable soroban-sdk releases from version 20.0.0 onwards.

## Related Documentation

- [Verification Troubleshooting Guide](./VERIFICATION_TROUBLESHOOTING.md)
- [Error Codes Reference](./ERROR_CODES.md)
- [Security Best Practices](./SECURITY.md)

## Support

For verification issues:
- GitHub Issues: https://github.com/ALIPHATICHYD/Soroban-Registry/issues
- Tag with: `verification`, `contracts`
