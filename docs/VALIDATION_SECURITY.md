# API Input Validation and Security Documentation

## Overview

The Soroban Registry API includes comprehensive input validation and security middleware to prevent injection attacks, malformed data, and denial-of-service attempts. This document describes the validation system and how to use it.

## Architecture

The validation system consists of five key components:

### 1. Validators (`validation/validators.rs`)
Reusable validation functions for common input types:
- **Contract IDs**: Stellar format validation (C followed by 55 base32 characters)
- **Stellar Addresses**: Stellar format validation (G followed by 55 base32 characters)
- **Semantic Versions**: Strict X.Y.Z format with optional pre-release and build metadata
- **URLs**: Format validation with optional restrictions
- **Text Fields**: Length constraints, HTML detection, XSS pattern detection
- **Tags**: Count and length limits with XSS validation

### 2. Sanitizers (`validation/sanitizers.rs`)
Functions to clean and normalize input data:
- Trim whitespace
- Strip HTML tags
- Remove control characters
- Normalize whitespace
- Escape special characters for display

### 3. URL Validation (`validation/url_validation.rs`)
Secure URL handling with:
- HTTPS-only enforcement
- Domain whitelist support
- URL component parsing
- Safe normalization

### 4. Payload Size Validation (`validation/payload_size.rs`)
Middleware that:
- Checks Content-Length headers
- Enforces maximum request body size (default: 5 MB)
- Returns 413 Payload Too Large for oversized requests
- Logs violations for security monitoring

### 5. Validation Failure Rate Limiting (`validation/validation_rate_limit.rs`)
Rate limiting for validation failures:
- Tracks failures per IP address
- Configurable failure threshold (default: 20 per 60 seconds)
- Returns 429 Too Many Requests when exceeded
- Prevents attackers from probing the API

### 6. Security Logging (`security_log.rs`)
Structured logging for security events:
- Validation failures per field
- Payload size violations
- Rate limit violations
- Suspicious patterns and injection attempts
- Client IP tracking for forensics

## Validation Rules

### Contract ID
- Format: Exactly 56 characters
- Pattern: `^C[A-Z0-9]{55}$`
- Example: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`

### Stellar Address
- Format: Exactly 56 characters
- Pattern: `^G[A-Z0-9]{55}$`
- Example: `GBRPYHIL2CI3WHZSRXG5ZRAML54KVXVP5JUDLHTCHHYDAYCOPREA5Z5L`

### Semantic Version
- Format: `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`
- Examples: `1.0.0`, `2.1.0-alpha`, `1.0.0-rc.1+build.123`

### URLs
- Protocol: **HTTPS only** (HTTP is rejected)
- Domain whitelist (configurable via `ALLOWED_DOMAINS` environment variable)
- Default whitelisted domains:
  - github.com
  - gitlab.com
  - gitea.com
  - git.stellar.org
  - soroban.stellar.org
  - docs.rs
  - crates.io
  - raw.githubusercontent.com

### Text Fields
- Maximum length: 5000 characters
- HTML tags stripped automatically
- Control characters removed
- Whitespace normalized

## Usage Patterns

### Using ValidatedJson in Handlers

```rust
use crate::validation::{ValidatedJson, Validatable, ValidationBuilder};

#[derive(serde::Deserialize)]
pub struct PublishContractRequest {
    pub contract_id: String,
    pub version: String,
    pub source_url: String,
    pub description: String,
}

impl Validatable for PublishContractRequest {
    fn sanitize(&mut self) {
        self.contract_id = sanitize_contract_id(&self.contract_id);
        self.version = trim(&self.version);
        self.source_url = trim(&self.source_url);
        self.description = sanitize_description(&self.description);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        ValidationBuilder::new()
            .check("contract_id", || validate_contract_id(&self.contract_id))
            .check("version", || validate_semver(&self.version))
            .check("source_url", || validate_url_https_only_with_whitelist(&self.source_url))
            .check("description", || validate_length(&self.description, 10, 5000))
            .build()
    }
}

#[post("/api/contracts/publish")]
pub async fn publish_contract(
    ValidatedJson(req): ValidatedJson<PublishContractRequest>,
) -> impl IntoResponse {
    // req is already sanitized and validated
    // Store to database, etc.
}
```

## Configuration

### Environment Variables

```bash
# Payload size limits
MAX_PAYLOAD_SIZE_MB=5                          # Default: 5 MB

# Validation failure rate limiting
VALIDATION_FAILURE_LIMIT=20                    # Failures before rate limit
VALIDATION_FAILURE_WINDOW_SECONDS=60           # Time window in seconds

# URL domain whitelist (comma-separated)
ALLOWED_DOMAINS="github.com,gitlab.com,docs.rs"
```

## Error Responses

### Validation Error (400 Bad Request)

```json
{
  "error": "ValidationError",
  "message": "Validation failed for 2 fields",
  "code": 400,
  "errors": [
    {
      "field": "contract_id",
      "message": "must be a valid Stellar contract ID"
    },
    {
      "field": "version",
      "message": "must be a valid semantic version"
    }
  ],
  "timestamp": "2026-02-25T10:30:00Z",
  "correlation_id": "uuid-here"
}
```

### Payload Too Large (413)

```json
{
  "error": "PayloadTooLarge",
  "message": "Request payload exceeds maximum size of 5 MB (5242880 bytes)",
  "code": 413,
  "max_size_mb": 5,
  "max_size_bytes": 5242880,
  "timestamp": "2026-02-25T10:30:00Z",
  "correlation_id": "uuid-here"
}
```

### Validation Failure Rate Limited (429)

```json
{
  "error": "TooManyValidationFailures",
  "message": "Too many validation failures. Please try again in 60 seconds",
  "code": 429,
  "retry_after_seconds": 60,
  "correlation_id": "uuid-here",
  "timestamp": "2026-02-25T10:30:00Z"
}
```

## Security Logging

All validation failures are logged with structured data for security monitoring:

```json
{
  "event_type": "validation_failed",
  "client_ip": "192.168.1.100",
  "field": "contract_id",
  "message": "must be a valid Stellar contract ID",
  "path": "/api/contracts",
  "method": "POST",
  "failure_count": 3,
  "correlation_id": "abc-123-def",
  "timestamp": "2026-02-25T10:30:00.123Z"
}
```

## Acceptance Criteria

### ✅ Invalid Contract ID Returns 400 with Clear Error
```bash
curl -X POST http://localhost:3001/api/contracts \
  -H "Content-Type: application/json" \
  -d '{"contract_id": "INVALID"}'

# Response:
# {
#   "error": "ValidationError",
#   "code": 400,
#   "errors": [{
#     "field": "contract_id",
#     "message": "must be a valid Stellar contract ID"
#   }]
# }
```

### ✅ HTML in Text Fields Stripped, Not Stored
```bash
curl -X POST http://localhost:3001/api/contracts \
  -H "Content-Type: application/json" \
  -d '{"description": "<script>alert(1)</script>Safe content"}'

# Text is sanitized: "Safe content" (HTML stripped)
```

### ✅ Oversized Payloads Rejected with 413
```bash
# Send 10 MB payload (default limit is 5 MB)
curl -X POST http://localhost:3001/api/contracts \
  -H "Content-Type: application/json" \
  -d @large-file.json

# Response: 413 Payload Too Large
```

### ✅ Stellar Address Validation Catches Typos
```bash
# Single character typo
curl -X POST http://localhost:3001/api/publishers \
  -H "Content-Type: application/json" \
  -d '{"address": "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAH"}'

# Response: "must be a valid Stellar address"
```

### ✅ Security Logs Track Validation Failures per IP
- View logs in your ELK/Splunk instance
- Filter by `client_ip` or `event_type: validation_failed`
- Monitor for repeated failures from same IP
- Automatic rate limiting prevents brute force

## Testing

Run the comprehensive validation tests:

```bash
cd backend/api
cargo test validation:: -- --nocapture   # Run all validation tests
cargo test comprehensive_tests           # Run acceptance criteria tests
```

Test files:
- Unit tests: `src/validation/validators.rs`, `src/validation/sanitizers.rs`
- Integration tests: `src/validation/comprehensive_tests.rs`
- Security: `src/security_log.rs` (logging tests)

## Security Best Practices

1. **Always use ValidatedJson extractor** - Never use raw `Json<T>` for user input
2. **Sanitize before storing** - The `Validatable` trait ensures this
3. **Log all failures** - Security events are automatically logged with client IP
4. **Monitor rate limits** - Watch for repeated validation failures from same IP
5. **Update domain whitelist** - Configure `ALLOWED_DOMAINS` for your sources
6. **Set appropriate size limits** - Adjust `MAX_PAYLOAD_SIZE_MB` for your needs

## Troubleshooting

### "must be a valid Stellar contract ID"
- Check the ID is exactly 56 characters
- Verify it starts with 'C' (not 'G' which is an address)
- Ensure all characters are uppercase alphanumerics

### "URLs must use HTTPS protocol"
- Change `http://` URLs to `https://`
- Only HTTPS is allowed for security

### "Domain is not whitelisted"
- Add the domain to `ALLOWED_DOMAINS` environment variable
- Format: comma-separated list of domains
- Subdomains must be explicitly added or use wildcard (*.github.com)

### Rate limited (429 Too Many Requests)
- This indicates multiple invalid requests from the same IP
- Wait for the retry window (default 60 seconds)
- Check logs for what fields are failing validation

## References

- [Stellar Address Format](https://developers.stellar.org/docs/glossary#public-key)
- [Semantic Versioning](https://semver.org/)
- [OWASP Input Validation](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
