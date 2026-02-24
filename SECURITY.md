# Security Policy

## Reporting a Vulnerability

The Soroban Registry team takes security vulnerabilities seriously. We appreciate your efforts to responsibly disclose your findings.

### How to Report

**Please DO NOT report security vulnerabilities through public GitHub issues.**

Instead, report security vulnerabilities privately using one of these methods:

1. **GitHub Security Advisories** (Preferred)
   - Go to the [Security tab](https://github.com/ALIPHATICHYD/Soroban-Registry/security/advisories)
   - Click "Report a vulnerability"
   - Fill out the form with details

2. **Email**
   - Send an email to: `security@soroban-registry.example`
   - Use PGP encryption if possible (key below)
   - Include "SECURITY" in the subject line

3. **Private Disclosure via GitHub**
   - DM a maintainer on GitHub with details
   - Request private disclosure channel

### What to Include in Your Report

Please provide as much information as possible:

- **Type of vulnerability** (e.g., SQL injection, XSS, authentication bypass)
- **Affected component(s)** (API, frontend, indexer, etc.)
- **Steps to reproduce** (detailed, ideally with a proof-of-concept)
- **Impact assessment** (what an attacker could do)
- **Suggested fix** (if you have one)
- **Your contact information** (for follow-up questions)

### Example Report Template

```
**Summary**: Brief description of the vulnerability

**Affected Component**: API endpoint /api/contracts/verify

**Severity**: Critical / High / Medium / Low

**Description**: Detailed explanation of the vulnerability

**Steps to Reproduce**:
1. Send POST request to /api/contracts/verify
2. Include malicious payload in source_code field
3. Observe unvalidated execution

**Impact**: Remote code execution on verification server

**Proof of Concept**: [Include code, curl command, or screenshots]

**Suggested Fix**: Sanitize all user input before compilation

**Reporter Contact**: your-email@example.com
```

## Response Timeline

We are committed to responding quickly to security reports:

| Timeline | Action |
|----------|--------|
| **Within 24 hours** | Initial acknowledgment of your report |
| **Within 7 days** | Assessment of severity and impact |
| **Within 30 days** | Fix developed and tested |
| **Within 60 days** | Fix deployed and public disclosure (coordinated with you) |

**Note:** Timeline may vary based on complexity. We will keep you updated on progress.

## Severity Classification

We use the following severity levels:

### Critical
- Remote code execution
- Authentication bypass
- SQL injection with data exfiltration
- Privilege escalation to admin

**Response:** Emergency patch within 24-48 hours

### High
- XSS attacks
- CSRF vulnerabilities
- Exposed sensitive data
- Denial of service affecting all users

**Response:** Patch within 7 days

### Medium
- Information disclosure (non-sensitive)
- Missing security headers
- Rate limiting bypass
- Local file inclusion

**Response:** Patch within 30 days

### Low
- Non-exploitable information leakage
- Best practice violations
- Minor configuration issues

**Response:** Fix in next release cycle

## Disclosure Policy

We follow **coordinated disclosure**:

1. **Private disclosure**: We work with you privately to develop a fix
2. **Fix deployment**: We deploy the fix to production
3. **Public disclosure**: After the fix is live, we publish a security advisory
4. **Credit**: We credit you in the advisory (if desired)

**Disclosure timeline:** Typically 90 days from initial report, or sooner if all parties agree.

## Security Rewards

While we don't currently offer a formal bug bounty program, we deeply appreciate security researchers who help us improve our platform.

**Recognition:**
- Public credit in security advisory (if desired)
- Listed in Hall of Fame (coming soon)
- Swag and merchandise (for significant findings)
- Potential monetary reward for critical vulnerabilities (case-by-case basis)

## Out of Scope

The following are **not** considered security vulnerabilities:

- Denial of service via rate limit testing (use small scale only)
- Reports from automated scanners without validation
- Social engineering attacks on team members
- Physical attacks on infrastructure
- Issues in third-party dependencies (unless we're not patching)
- Missing security headers that don't lead to exploits
- Descriptive error messages without sensitive data exposure
- Logout CSRF
- Self-XSS
- Issues requiring MITM attacks on users without TLS

## Security Best Practices for Users

If you're using the Soroban Registry API or deploying contracts:

- **Never commit secrets** (API keys, private keys) to source code
- **Use environment variables** for configuration
- **Enable rate limiting** on your applications
- **Validate all inputs** before sending to API
- **Use HTTPS** for all API requests
- **Rotate API keys** regularly
- **Audit smart contracts** before deploying
- **Monitor for anomalies** in contract interactions

See [Security Best Practices Documentation](./docs/SECURITY.md) for detailed guidance.

## PGP Public Key

For encrypted communications:

```
-----BEGIN PGP PUBLIC KEY BLOCK-----

[PGP key would be here in production]

-----END PGP PUBLIC KEY BLOCK-----
```

## Security Advisories

Past security advisories are published at:
- https://github.com/ALIPHATICHYD/Soroban-Registry/security/advisories

Subscribe to notifications to stay informed.

## Contact

- **Security Email**: security@soroban-registry.example
- **General Contact**: support@soroban-registry.example
- **GitHub**: https://github.com/ALIPHATICHYD/Soroban-Registry

## Acknowledgments

We thank the following security researchers for their responsible disclosures:

*(List will be populated as vulnerabilities are reported and fixed)*

---

**Last Updated:** 2026-02-24

Thank you for helping keep Soroban Registry and the Stellar ecosystem secure!
