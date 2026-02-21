# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-21

### Added

- Initial release of soroban-registry
- 20+ lint rules for Soroban smart contract analysis:
  - `missing_error_handling`: Detects unwrap/expect in non-test code
  - `unsafe_unwrap`: Warns about unwrap in public functions
  - `integer_overflow`: Detects unchecked arithmetic
  - `reentrancy`: Warns about cross-contract calls before state writes
  - `storage_key_collision`: Detects duplicate storage keys
  - `missing_auth_check`: Ensures public state-changing functions have auth checks
  - `unbounded_loops`: Detects loops without break conditions
  - `hardcoded_addresses`: Warns about hardcoded contract addresses
  - `deprecated_api_usage`: Detects use of deprecated SDK functions
  - `large_data_in_storage`: Warns about unbounded collections in storage
  - `missing_events`: Reminds to emit events on state changes
  - `inefficient_clones`: Detects redundant clone calls
  - `public_fn_no_doc`: Warns about undocumented public functions
  - `unchecked_arithmetic`: Detects arithmetic without checked variants
  - `direct_storage_clear`: Warns about storage operations without validation
  - `panic_in_contract`: Prevents panic! calls in contracts
  - `missing_access_control`: Detects missing admin checks
  - `type_confusion`: Warns about unsafe type casts
  - `improper_token_handling`: Detects token transfers without validation
  - `unused_variables`: Warns about unused variable bindings

- CLI with multiple commands:
  - `lint`: Analyze contracts for issues
  - `rules`: List all available lint rules

- Configuration system with `.soroban-lint.toml`:
  - Per-rule severity override
  - Path-based ignore patterns
  - Default configurations

- Output formats:
  - Human-readable console output with color coding
  - JSON output for CI/CD integration
  - LSP-compatible diagnostic format

- Auto-fix support (`--fix` flag) for safe corrections

- Integration templates:
  - Git pre-commit hook for automatic linting before commits
  - GitHub Actions workflow for CI/CD

- Documentation:
  - Comprehensive README with usage examples
  - Configuration guide
  - Contributing guidelines

### Features

- Parallel rule execution using rayon for better performance
- Single-pass AST parsing per file
- Fast linting: <10s for 1000-line contracts
- Color-coded output for better readability
- Exit codes for CI/CD integration
- Configurable severity levels per rule

## [Unreleased]

### Planned

- [ ] VS Code extension for inline diagnostics
- [ ] Formal verification integration
- [ ] Performance caching with file hash tracking
- [ ] Custom rule plugin system
- [ ] LSP server for editor integration
- [ ] Web-based UI dashboard
- [ ] Advanced data flow analysis
- [ ] Machine learning-based anomaly detection
