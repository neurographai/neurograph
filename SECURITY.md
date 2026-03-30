# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.x.x   | Current development |
| < 0.1.0 | No longer supported |

## Reporting a Vulnerability

**DO NOT open a public GitHub issue for security vulnerabilities.**

Instead, please use one of these methods:

### 1. GitHub Security Advisories (Preferred)
Navigate to the [Security Advisories](https://github.com/neurographai/neurograph/security/advisories)
page and click "New draft security advisory".

### 2. Email
Send details to: **security@neurograph.dev**

### What to Include
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline
- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Fix Timeline**: Within 30 days for critical, 90 days for others
- **Disclosure**: Coordinated disclosure after fix is released

## Security Measures

### Build Security
- All CI builds use pinned action versions (SHA hashes)
- Dependencies audited via `cargo-audit` and `cargo-deny`
- SBOM generated for each release
- Container images scanned with Trivy

### Runtime Security
- Docker images run as non-root user
- No secrets embedded in images
- API endpoints require authentication (when enabled)
- Input sanitization on all user-facing endpoints
- SQL/Cypher injection prevention in graph queries

### Supply Chain
- Dependabot monitors all dependency updates
- OpenSSF Scorecard runs weekly
- CodeQL analysis on every PR
- Signed releases (coming in v0.3)
