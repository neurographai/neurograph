# Security Policy

NeuroGraph takes security seriously. This document describes our security practices, supported versions, vulnerability reporting process, and the security measures built into the project.

---

## Table of Contents

- [Supported Versions](#supported-versions)
- [Reporting a Vulnerability](#reporting-a-vulnerability)
  - [How to Report](#how-to-report)
  - [What to Include](#what-to-include)
  - [Response Timeline](#response-timeline)
  - [Safe Harbor](#safe-harbor)
- [Security Measures](#security-measures)
  - [Build Security](#build-security)
  - [Dependency Management](#dependency-management)
  - [Runtime Security](#runtime-security)
  - [API Security](#api-security)
  - [Data Security](#data-security)
  - [Supply Chain Security](#supply-chain-security)
- [Security Architecture](#security-architecture)
  - [Threat Model](#threat-model)
  - [Trust Boundaries](#trust-boundaries)
  - [Attack Surface](#attack-surface)
- [Security Scanning](#security-scanning)
- [Incident Response](#incident-response)
- [Security Roadmap](#security-roadmap)
- [Hall of Fame](#hall-of-fame)

---

## Supported Versions

| Version | Supported | Notes |
|---------|-----------|-------|
| 0.1.x | ✅ Current development | Active security patches |
| < 0.1.0 | ❌ Unsupported | No patches; please upgrade |

Once NeuroGraph reaches v1.0, we will maintain security patches for the latest two minor versions.

---

## Reporting a Vulnerability

### How to Report

**DO NOT open a public GitHub issue for security vulnerabilities.** Public disclosure before a fix is available puts all users at risk.

Use one of these methods:

#### 1. GitHub Security Advisories (Preferred)

Navigate to the [Security Advisories](https://github.com/neurographai/neurograph/security/advisories) page and click **"New draft security advisory"**. This creates a private space where you and the maintainers can discuss the vulnerability.

#### 2. Email

Send a detailed report to: **security@neurograph.dev**

If you need to share sensitive information, use our PGP key (available on the Security Advisories page).

### What to Include

Please provide as much of the following as possible:

| Field | Description |
|-------|-------------|
| **Summary** | One-paragraph description of the vulnerability |
| **Affected component** | Which module/file is affected (e.g., `ingestion/pipeline.rs`) |
| **Affected versions** | Which versions are vulnerable |
| **Steps to reproduce** | Detailed reproduction steps (ideally minimal code) |
| **Impact assessment** | What can an attacker do? (data exfiltration, DoS, RCE, etc.) |
| **Severity estimate** | Your assessment: Low / Medium / High / Critical |
| **Suggested fix** | If you have one — patches are welcome |
| **CVE ID** | If you've already requested one |

### Response Timeline

| Stage | Timeframe | Description |
|-------|-----------|-------------|
| **Acknowledgment** | Within 48 hours | We confirm receipt of your report |
| **Initial triage** | Within 7 days | We assess severity and assign to a maintainer |
| **Fix development** | 14-30 days (critical), 30-90 days (others) | Patch developed and tested |
| **Notification** | Before public release | Reporter is notified and can review the fix |
| **Public disclosure** | After fix is released | CVE published, advisory updated |
| **Credit** | In release notes | Reporter credited (unless they prefer anonymity) |

### Safe Harbor

We consider security research conducted in good faith to be authorized. We will not pursue legal action against researchers who:

- Act in good faith to avoid privacy violations, data destruction, and service disruption
- Report vulnerabilities through the channels described above
- Provide reasonable time for remediation before public disclosure
- Do not access or modify data belonging to other users

---

## Security Measures

### Build Security

| Measure | Implementation |
|---------|---------------|
| **Pinned CI actions** | All GitHub Actions use SHA-pinned versions, not tags |
| **Dependency audit** | `cargo audit` runs in CI on every PR |
| **License compliance** | `cargo deny check` validates no problematic licenses |
| **SBOM generation** | CycloneDX SBOM generated for each release |
| **Container scanning** | Docker images scanned with Trivy before publish |
| **Reproducible builds** | `Cargo.lock` committed, exact dependency versions locked |

### Dependency Management

NeuroGraph follows strict dependency hygiene:

```toml
# deny.toml — cargo-deny configuration
[advisories]
vulnerability = "deny"      # Fail on known vulnerabilities
unmaintained = "warn"        # Warn on unmaintained dependencies

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Unicode-DFS-2016"]
unlicensed = "deny"
copyleft = "deny"

[bans]
multiple-versions = "warn"   # Flag multiple versions of same crate
```

**Automated updates:**
- **Dependabot** monitors all Rust and npm dependencies
- Security patches are auto-merged if CI passes
- Minor/major updates require manual review

### Runtime Security

| Measure | Details |
|---------|---------|
| **Non-root containers** | Docker images run as UID 1000 (non-root) |
| **No embedded secrets** | No API keys, tokens, or credentials in images |
| **Memory safety** | Rust eliminates buffer overflows, use-after-free, data races |
| **No unsafe code** | The core crate has zero `unsafe` blocks |
| **Input validation** | All user-facing inputs are validated and sanitized |
| **Error redaction** | Internal errors are never exposed to API consumers |

### API Security

| Feature | Status | Details |
|---------|--------|---------|
| **Authentication** | Optional | Bearer token authentication when enabled |
| **Rate limiting** | ✅ | Configurable per-endpoint rate limits |
| **CORS** | ✅ | Strict origin whitelisting |
| **Request size limits** | ✅ | Max payload size: 10MB (configurable) |
| **Input sanitization** | ✅ | All text inputs sanitized before graph operations |
| **Injection prevention** | ✅ | Parameterized queries for all database operations |
| **TLS** | ✅ | HTTPS enforced in production deployments |

### Data Security

| Feature | Details |
|---------|---------|
| **At-rest encryption** | sled supports OS-level encryption; Neo4j supports built-in encryption |
| **In-transit encryption** | All API communication over TLS |
| **Data isolation** | `group_id` provides tenant-level data isolation |
| **No telemetry** | NeuroGraph sends no data to external services (unless LLM is configured) |
| **LLM data** | Text sent to LLMs (OpenAI, etc.) is subject to their data policies |

### Supply Chain Security

| Measure | Implementation |
|---------|---------------|
| **OpenSSF Scorecard** | Runs weekly, score visible in README badge |
| **CodeQL analysis** | Runs on every PR (Rust + TypeScript) |
| **Signed releases** | Coming in v0.3 (sigstore cosign) |
| **Provenance** | SLSA provenance attestations planned for v0.3 |
| **CODEOWNERS** | All changes to security-critical paths require maintainer review |

---

## Security Architecture

### Threat Model

NeuroGraph is designed for the following deployment scenarios:

| Scenario | Trust Level | Key Threats |
|----------|------------|-------------|
| **Embedded library** | High (single-process) | Dependency vulnerabilities, memory corruption |
| **Local API server** | Medium (local network) | CSRF, unauthorized access, DoS |
| **Cloud deployment** | Lower (public internet) | All of the above + injection, data exfiltration |

### Trust Boundaries

```
┌─────────────────────────────────────────┐
│            Untrusted Zone               │
│   User input, API requests, LLM output  │
├─────────────────────────────────────────┤
│         Validation Layer                │
│   Input sanitization, auth, rate limit  │
├─────────────────────────────────────────┤
│            Trusted Zone                 │
│   Core engine, graph driver, storage    │
├─────────────────────────────────────────┤
│        External Services                │
│   LLM APIs, Neo4j, embedding APIs       │
│   (treated as semi-trusted)             │
└─────────────────────────────────────────┘
```

### Attack Surface

| Surface | Risk | Mitigation |
|---------|------|------------|
| **Text ingestion** | Prompt injection via ingested text | Input sanitization, LLM output validation |
| **REST API** | Standard web attacks (XSS, CSRF, injection) | Input validation, CORS, CSP headers |
| **Graph queries** | Cypher/query injection (Neo4j backend) | Parameterized queries, never string interpolation |
| **LLM responses** | Hallucinated or manipulated extraction results | Confidence scoring, validator pipeline |
| **Docker images** | Vulnerable base image, embedded secrets | Trivy scanning, multi-stage builds, non-root |
| **Dependencies** | Known CVEs in transitive dependencies | cargo-audit, Dependabot, cargo-deny |

---

## Security Scanning

### Automated (CI)

| Tool | What It Checks | Frequency |
|------|---------------|-----------|
| `cargo audit` | Known vulnerabilities in Rust deps | Every PR |
| `cargo deny` | License + vulnerability policy | Every PR |
| CodeQL | Semantic code analysis (Rust + TS) | Every PR |
| Trivy | Container image vulnerabilities | Every Docker build |
| OpenSSF Scorecard | Project security posture | Weekly |
| Dependabot | Outdated dependencies | Daily |

### Manual

- **Penetration testing:** Planned for post-v1.0 release
- **Threat modeling updates:** Reviewed quarterly
- **Dependency deep-dive:** Major dependency upgrades trigger manual security review

---

## Incident Response

If a security incident occurs:

1. **Contain** — Disable affected endpoints/features
2. **Assess** — Determine scope and severity
3. **Fix** — Develop and test patch
4. **Notify** — Inform affected users via GitHub Advisory
5. **Publish** — Release patched version with CVE
6. **Post-mortem** — Document root cause and preventive measures

---

## Security Roadmap

| Feature | Target Version | Status |
|---------|---------------|--------|
| Input sanitization (all endpoints) | v0.1.0 | ✅ Complete |
| Non-root Docker images | v0.1.0 | ✅ Complete |
| cargo-audit in CI | v0.1.0 | ✅ Complete |
| CodeQL analysis | v0.1.0 | ✅ Complete |
| OpenSSF Scorecard | v0.1.0 | ✅ Complete |
| Signed releases (cosign) | v0.3.0 | 🔄 Planned |
| SLSA provenance | v0.3.0 | 🔄 Planned |
| API authentication (JWT) | v0.4.0 | 🔄 Planned |
| Penetration testing | v1.0.0 | 📋 Roadmap |
| SOC 2 compliance documentation | v1.0.0+ | 📋 Roadmap |

---

## Hall of Fame

We gratefully acknowledge security researchers who have responsibly disclosed vulnerabilities:

*This section will be populated as reports come in. Be the first!*

---

*For development practices, see [DEVELOPING.md](DEVELOPING.md). For contributing, see [CONTRIBUTING.md](CONTRIBUTING.md).*
