# Security Best Practices

## Overview

This document covers security best practices for using the Soroban Registry API, deploying smart contracts, and handling sensitive data. Following these guidelines helps protect your applications, users, and the broader Stellar ecosystem.

## Table of Contents

1. [Authentication & Authorization](#authentication--authorization)
2. [API Security](#api-security)
3. [Smart Contract Security](#smart-contract-security)
4. [Data Protection](#data-protection)
5. [Secrets Management](#secrets-management)
6. [Input Validation](#input-validation)
7. [Rate Limiting & DDoS Protection](#rate-limiting--ddos-protection)
8. [Security Headers](#security-headers)
9. [Encryption](#encryption)
10. [Pre-Deployment Security Checklist](#pre-deployment-security-checklist)

---

## Authentication & Authorization

### Current Authentication Model

Currently, the Soroban Registry API is **publicly accessible** without authentication for read operations. Future versions will implement API key authentication.

### Planned Authentication (Coming Soon)

**API Key Authentication:**

```http
GET /api/contracts/{id}
Authorization: Bearer your-api-key-here
```

**Best Practices:**
- Store API keys in environment variables, never in code
- Rotate keys every 90 days
- Use different keys for development and production
- Revoke compromised keys immediately
- Never share keys in public repositories or logs

### Authorization Model

The registry implements role-based access control (RBAC):

| Role | Permissions |
|------|-------------|
| **Anonymous** | Read public contracts, search, view verified source |
| **Registered User** | Publish contracts, verify contracts, manage own contracts |
| **Publisher** | Publish under verified identity, manage publisher profile |
| **Admin** | Manage all contracts, users, moderate content |

**Principle of Least Privilege**: Always use the minimum permissions necessary for your use case.

---

## API Security

### Secure API Usage

#### 1. Always Use HTTPS

**Never** send requests over HTTP in production:

```javascript
// ✅ Correct
const API_URL = 'https://registry.soroban.example';

// ❌ Wrong
const API_URL = 'http://registry.soroban.example';
```

#### 2. Validate TLS Certificates

Ensure your HTTP client validates TLS certificates:

```python
import requests

# ✅ Correct (default)
response = requests.get(url, verify=True)

# ❌ Wrong - disables certificate verification
response = requests.get(url, verify=False)
```

#### 3. Set Request Timeouts

Always set timeouts to prevent hanging requests:

```python
import requests

response = requests.get(
    url,
    timeout=30  # 30 second timeout
)
```

```javascript
const response = await fetch(url, {
  signal: AbortSignal.timeout(30000) // 30 second timeout
});
```

#### 4. Sanitize User Input

Never directly interpolate user input into API requests:

```python
# ❌ Wrong - vulnerable to injection
contract_id = user_input
url = f"https://api.example/contracts/{contract_id}"

# ✅ Correct - validate first
import re
if re.match(r'^C[A-Z0-9]{55}$', user_input):
    contract_id = user_input
    url = f"https://api.example/contracts/{contract_id}"
else:
    raise ValueError("Invalid contract ID")
```

#### 5. Handle Errors Securely

Don't expose sensitive information in error messages:

```python
# ❌ Wrong - exposes sensitive data
except Exception as e:
    return f"Error: {e}, API_KEY: {API_KEY}, DB_CONN: {DB_CONN}"

# ✅ Correct - sanitize error messages
except Exception as e:
    logger.error(f"API error: {e}", extra={"correlation_id": correlation_id})
    return {"error": "An error occurred. Please try again.", "correlation_id": correlation_id}
```

---

## Smart Contract Security

### Before Publishing Contracts

#### 1. Code Review

- Have contracts reviewed by security experts
- Use automated tools (soroban-lint)
- Follow Stellar smart contract best practices

#### 2. Audit for Common Vulnerabilities

Run soroban-lint before publishing:

```bash
soroban-lint scan src/ --all-rules
```

**Common vulnerabilities to check:**

| Vulnerability | Risk | Prevention |
|---------------|------|------------|
| Missing auth checks | Unauthorized access | Use `env.current_contract_address()` checks |
| Integer overflow | Incorrect calculations | Use checked arithmetic |
| Reentrancy | State corruption | Use checks-effects-interactions pattern |
| Unbounded loops | DoS via gas exhaustion | Set maximum iteration limits |
| Unsafe unwrap | Panic/contract halt | Use `?` operator or proper error handling |

#### 3. Test Thoroughly

```bash
# Run all tests
cargo test

# Test with fuzzing
cargo fuzz

# Integration tests on testnet
soroban contract invoke --id $CONTRACT --network testnet
```

#### 4. Verify Contract Source

Always verify your contracts after deployment:

```bash
soroban-registry verify \
  --contract-id $CONTRACT_ID \
  --source ./src \
  --network mainnet
```

**Why?** Verification proves to users that the deployed bytecode matches the public source code.

### Don't Include Secrets in Contracts

```rust
// ❌ Wrong - hardcoded secrets
const ADMIN_KEY: &str = "SDFY3LK...PRIVATE_KEY";

// ✅ Correct - pass as parameter
pub fn admin_action(env: Env, admin: Address) {
    admin.require_auth();
    // ...
}
```

### Input Validation in Contracts

```rust
pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
    // ✅ Validate inputs
    if amount <= 0 {
        panic!("Amount must be positive");
    }

    if from == to {
        panic!("Cannot transfer to self");
    }

    // Proceed with transfer
}
```

---

## Data Protection

### Personal Data Handling

If your application collects personal data through the registry:

1. **Minimize data collection** - only collect what's necessary
2. **Encrypt sensitive data** at rest and in transit
3. **Implement data retention policies** - delete old data
4. **Provide data export/deletion** - user rights (GDPR)
5. **Log access to personal data** - audit trail

### Data Classification

| Classification | Examples | Protection Required |
|----------------|----------|---------------------|
| **Public** | Contract addresses, verified source code | None |
| **Internal** | API metrics, usage statistics | Access control |
| **Confidential** | User emails, API keys | Encryption + access control |
| **Secret** | Private keys, database passwords | HSM/vault + strict access |

---

## Secrets Management

### Never Commit Secrets to Git

```bash
# Add to .gitignore
.env
.env.local
*.key
*.pem
secrets/
```

### Use Environment Variables

```bash
# .env (never commit this file)
DATABASE_URL=postgresql://user:password@localhost/db
API_KEY=your-api-key-here
STELLAR_SECRET_KEY=SDFY3LK...
```

```python
import os
from dotenv import load_dotenv

load_dotenv()

DATABASE_URL = os.getenv('DATABASE_URL')
API_KEY = os.getenv('API_KEY')
```

### Use Secret Management Tools

**For Production:**
- **AWS Secrets Manager** - for AWS deployments
- **HashiCorp Vault** - for self-hosted
- **Azure Key Vault** - for Azure deployments
- **Google Secret Manager** - for GCP deployments

**Example with HashiCorp Vault:**

```python
import hvac

client = hvac.Client(url='https://vault.example.com')
client.token = os.getenv('VAULT_TOKEN')

secret = client.secrets.kv.v2.read_secret_version(path='soroban/api-key')
API_KEY = secret['data']['data']['key']
```

### Rotate Secrets Regularly

- **API Keys**: Every 90 days
- **Database Passwords**: Every 180 days
- **TLS Certificates**: Before expiration (typically 90 days)
- **Signing Keys**: Every 365 days

---

## Input Validation

### Validate All User Input

**Never trust user input**. Always validate on the server side.

#### Contract ID Validation

```python
import re

def validate_contract_id(contract_id: str) -> bool:
    """
    Stellar contract IDs are 56 characters, starting with 'C'
    """
    pattern = r'^C[A-Z2-7]{55}$'
    return bool(re.match(pattern, contract_id))

# Usage
if not validate_contract_id(user_input):
    raise ValueError("Invalid contract ID format")
```

#### Network Validation

```python
VALID_NETWORKS = {'mainnet', 'testnet', 'futurenet'}

def validate_network(network: str) -> bool:
    return network.lower() in VALID_NETWORKS
```

#### Pagination Validation

```python
def validate_pagination(limit: int, offset: int):
    if not (1 <= limit <= 1000):
        raise ValueError("Limit must be between 1 and 1000")
    if offset < 0:
        raise ValueError("Offset must be non-negative")
```

### Prevent XSS (Cross-Site Scripting)

When displaying user-generated content (contract names, descriptions):

```javascript
// ✅ React automatically escapes
function ContractCard({ name, description }) {
  return (
    <div>
      <h2>{name}</h2>  {/* Automatically escaped */}
      <p>{description}</p>
    </div>
  );
}

// ❌ Dangerous - can inject scripts
function ContractCard({ name, description }) {
  return (
    <div dangerouslySetInnerHTML={{ __html: description }} />
  );
}
```

**For HTML sanitization:**

```javascript
import DOMPurify from 'dompurify';

const cleanHtml = DOMPurify.sanitize(userInput);
```

### Prevent SQL Injection

Always use parameterized queries:

```python
# ❌ Wrong - vulnerable to SQL injection
query = f"SELECT * FROM contracts WHERE id = '{contract_id}'"
result = db.execute(query)

# ✅ Correct - parameterized query
query = "SELECT * FROM contracts WHERE id = $1"
result = await db.fetch_one(query, contract_id)
```

```rust
// ✅ Correct with sqlx
let contract = sqlx::query_as::<_, Contract>(
    "SELECT * FROM contracts WHERE contract_id = $1"
)
.bind(contract_id)
.fetch_one(&pool)
.await?;
```

---

## Rate Limiting & DDoS Protection

### Respect Rate Limits

See [API Rate Limiting](./API_RATE_LIMITING.md) for details.

**Key points:**
- Monitor `X-RateLimit-Remaining` header
- Implement exponential backoff on 429 errors
- Cache responses when possible
- Use batch endpoints for bulk operations

### Implement Client-Side Rate Limiting

```javascript
class RateLimiter {
  constructor(maxRequests, perMilliseconds) {
    this.max = maxRequests;
    this.window = perMilliseconds;
    this.requests = [];
  }

  async acquire() {
    const now = Date.now();
    this.requests = this.requests.filter(t => t > now - this.window);

    if (this.requests.length >= this.max) {
      const oldestRequest = this.requests[0];
      const waitTime = oldestRequest + this.window - now;
      await new Promise(resolve => setTimeout(resolve, waitTime));
      return this.acquire();
    }

    this.requests.push(now);
  }
}

// Usage: 100 requests per minute
const limiter = new RateLimiter(100, 60000);

async function apiCall(url) {
  await limiter.acquire();
  return fetch(url);
}
```

---

## Security Headers

### CORS Configuration

Configure CORS properly to prevent unauthorized cross-origin requests:

```rust
// Backend CORS configuration
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin("https://app.example.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
```

**Production**: Whitelist specific origins
**Development**: Can use `allow_origin(Any)` but never in production

### Content Security Policy (CSP)

Add CSP headers to frontend:

```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self';
               script-src 'self' 'unsafe-inline';
               style-src 'self' 'unsafe-inline';
               img-src 'self' data: https:;
               connect-src 'self' https://registry.soroban.example;">
```

### Other Security Headers

```http
# Prevent clickjacking
X-Frame-Options: DENY

# Prevent MIME sniffing
X-Content-Type-Options: nosniff

# Enable XSS protection
X-XSS-Protection: 1; mode=block

# Force HTTPS
Strict-Transport-Security: max-age=31536000; includeSubDomains

# Referrer policy
Referrer-Policy: strict-origin-when-cross-origin
```

---

## Encryption

### Data at Rest

**Database encryption:**
- Use encrypted storage volumes (AWS EBS encryption, etc.)
- Enable database-level encryption (PostgreSQL pgcrypto)
- Encrypt sensitive columns individually

**Example: Encrypting API keys in database**

```python
from cryptography.fernet import Fernet

# Generate key (store securely, not in code!)
ENCRYPTION_KEY = os.getenv('ENCRYPTION_KEY')
cipher = Fernet(ENCRYPTION_KEY)

def encrypt_api_key(api_key: str) -> bytes:
    return cipher.encrypt(api_key.encode())

def decrypt_api_key(encrypted_key: bytes) -> str:
    return cipher.decrypt(encrypted_key).decode()
```

### Data in Transit

**Always use TLS 1.2 or higher:**

```nginx
# Nginx TLS configuration
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers HIGH:!aNULL:!MD5;
ssl_prefer_server_ciphers on;
```

**Certificate management:**
- Use Let's Encrypt for free TLS certificates
- Set up auto-renewal
- Monitor certificate expiration

---

## Pre-Deployment Security Checklist

Use this checklist before deploying to production:

### Application Security

- [ ] All secrets in environment variables (not hardcoded)
- [ ] `.env` and `.env.local` in `.gitignore`
- [ ] No sensitive data in logs
- [ ] All user input validated
- [ ] SQL injection prevented (parameterized queries)
- [ ] XSS prevention implemented
- [ ] CSRF protection enabled
- [ ] Rate limiting configured
- [ ] Security headers set (CSP, X-Frame-Options, etc.)
- [ ] CORS properly configured
- [ ] Error messages don't expose sensitive info

### Infrastructure Security

- [ ] TLS/HTTPS enforced
- [ ] TLS certificates valid and not expiring soon
- [ ] Database access restricted (firewall rules)
- [ ] No default passwords used
- [ ] SSH keys used (not passwords) for server access
- [ ] Firewall configured (only necessary ports open)
- [ ] Logging enabled for security events
- [ ] Automated backups configured
- [ ] Disaster recovery plan documented

### Smart Contract Security

- [ ] Contract code reviewed by security expert
- [ ] soroban-lint passed with no critical issues
- [ ] All tests passing
- [ ] Integration tests on testnet completed
- [ ] Contract verified on registry after deployment
- [ ] No hardcoded secrets in contract
- [ ] Authorization checks implemented
- [ ] Integer overflow protection
- [ ] Gas limits considered (unbounded loops avoided)

### Monitoring & Incident Response

- [ ] Security monitoring enabled (failed logins, unusual activity)
- [ ] Alerting configured for security events
- [ ] Incident response plan documented
- [ ] Security contact information public (SECURITY.md)
- [ ] Vulnerability disclosure process defined
- [ ] Logs retained for audit (30-90 days)

### Compliance & Documentation

- [ ] Privacy policy published (if collecting personal data)
- [ ] Terms of service published
- [ ] Security documentation up to date
- [ ] Dependency vulnerabilities scanned (`cargo audit`, `npm audit`)
- [ ] Third-party service security reviewed

---

## Code Review Security Guidelines

When reviewing code, check for:

### Authentication & Authorization

- [ ] All protected endpoints have auth checks
- [ ] User can only access their own resources
- [ ] Admin operations require admin role
- [ ] JWT tokens validated properly

### Input Validation

- [ ] All user input validated
- [ ] Type checking performed
- [ ] Range limits enforced
- [ ] No SQL injection vectors
- [ ] No command injection vectors

### Error Handling

- [ ] Errors don't expose sensitive data
- [ ] Stack traces not returned in production
- [ ] Correlation IDs used for debugging
- [ ] Errors logged securely

### Cryptography

- [ ] No custom crypto (use vetted libraries)
- [ ] Strong algorithms used (AES-256, RSA-2048+)
- [ ] Random number generation cryptographically secure
- [ ] Keys not hardcoded

### Dependencies

- [ ] No known vulnerabilities in dependencies
- [ ] Dependencies from trusted sources
- [ ] Minimal dependency tree (remove unused)
- [ ] Dependency versions pinned

---

## Dependency Scanning

Regularly scan for vulnerabilities:

### Rust

```bash
# Install cargo-audit
cargo install cargo-audit

# Scan for vulnerabilities
cargo audit

# Auto-fix (if available)
cargo audit fix
```

### JavaScript/Node.js

```bash
# Scan
npm audit

# Auto-fix
npm audit fix

# For production dependencies only
npm audit --production
```

### Python

```bash
# Install safety
pip install safety

# Scan
safety check

# Check specific file
safety check -r requirements.txt
```

---

## Security Updates

Keep all software up to date:

```bash
# Rust toolchain
rustup update

# Soroban CLI
cargo install soroban-cli --force

# Dependencies
cargo update
npm update
```

**Automated updates:**
- Use Dependabot (GitHub) for automated dependency updates
- Review and test updates before merging
- Subscribe to security mailing lists for Stellar/Soroban

---

## Reporting Security Issues

If you discover a security vulnerability:

1. **Do NOT** open a public GitHub issue
2. Follow the [Security Policy](../SECURITY.md) for responsible disclosure
3. Email: security@soroban-registry.example
4. Include detailed reproduction steps

---

## Related Documentation

- [Root Security Policy](../SECURITY.md) - Vulnerability reporting
- [API Rate Limiting](./API_RATE_LIMITING.md) - Rate limit configuration
- [Error Codes](./ERROR_CODES.md) - Error handling
- [Verification Workflow](./VERIFICATION_WORKFLOW.md) - Contract verification

---

## Security Resources

### Stellar/Soroban Security

- [Stellar Security Guide](https://developers.stellar.org/docs/learn/security)
- [Soroban Smart Contract Best Practices](https://soroban.stellar.org/docs/learn/security)

### General Security

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [CWE Top 25](https://cwe.mitre.org/top25/)

### Tools

- [soroban-lint](https://github.com/ALIPHATICHYD/Soroban-Registry/tree/main/soroban-registry/crates/soroban-lint-cli) - Soroban security linter
- [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit) - Rust dependency scanner
- [npm audit](https://docs.npmjs.com/cli/v8/commands/npm-audit) - Node.js dependency scanner

---

**Last Updated:** 2026-02-24

Stay secure! 🔒
