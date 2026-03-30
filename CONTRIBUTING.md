# Contributing to NeuroGraph

Thank you for considering contributing to NeuroGraph! Whether you're fixing a typo, reporting a bug, proposing a feature, or writing code — we appreciate your time and effort.

This guide will help you get started and understand how we work.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Getting Started](#getting-started)
  - [Fork & Clone](#fork--clone)
  - [Dev Environment Setup](#dev-environment-setup)
  - [Understanding the Codebase](#understanding-the-codebase)
- [Development Workflow](#development-workflow)
  - [Repository Structure](#repository-structure)
  - [Code Standards](#code-standards)
  - [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
  - [PR Checklist](#pr-checklist)
  - [What Gets Reviewed](#what-gets-reviewed)
  - [Review SLA](#review-sla)
- [Reporting Issues](#reporting-issues)
  - [Bug Reports](#bug-reports)
  - [Feature Requests](#feature-requests)
  - [Security Vulnerabilities](#security-vulnerabilities)
- [Good First Issues](#good-first-issues)
- [Architecture & Technical References](#architecture--technical-references)
- [Community](#community)
- [Recognition](#recognition)
- [License](#license)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code. Please report unacceptable behavior to **conduct@neurograph.dev**.

---

## How Can I Contribute?

| Contribution Type | Skill Level | Impact |
|-------------------|------------|--------|
| 🐛 **Bug reports** | Beginner | High — helps us find and fix issues |
| 📝 **Documentation** | Beginner | High — better docs help everyone |
| 🧪 **Test coverage** | Intermediate | High — prevents regressions |
| 🔧 **Bug fixes** | Intermediate | High — directly improves stability |
| ✨ **New features** | Advanced | Very high — grows the project |
| ⚡ **Performance** | Advanced | High — especially in hot paths |
| 🔒 **Security** | Advanced | Critical — keeps users safe |

Areas we especially welcome contributions in:
- Features marked **Experimental** or **Planned** in the README
- Storage backend implementations (Kuzu, FalkorDB)
- Python SDK (PyO3 bindings)
- TypeScript SDK (WASM-powered)
- Dashboard components and visualizations
- Benchmark suite expansion
- Documentation improvements

---

## Getting Started

### Fork & Clone

1. **Fork** the repository on GitHub: Click the "Fork" button at the top right of the [NeuroGraph repo](https://github.com/neurographai/neurograph).

2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/neurograph.git
   cd neurograph
   ```

3. **Add upstream** remote:
   ```bash
   git remote add upstream https://github.com/neurographai/neurograph.git
   ```

4. **Create a branch** for your change:
   ```bash
   git checkout -b feat/your-feature-name
   ```

### Dev Environment Setup

See [DEVELOPING.md](DEVELOPING.md) for full setup instructions. Quick start:

```bash
# Prerequisites: Rust 1.82+, Node.js 18+

# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Run dashboard
cd dashboard && npm install && npm run dev
```

### Understanding the Codebase

Before diving in, read these docs:

1. **[Architecture](docs/architecture.md)** — System design, data flow, module overview
2. **[Temporal Engine](docs/temporal.md)** — Bi-temporal model, forgetting, branching
3. **[Community Detection](docs/community.md)** — Louvain/Leiden algorithms, summarization
4. **[DEVELOPING.md](DEVELOPING.md)** — Build, test, debug instructions

The entry point to the codebase is `crates/neurograph-core/src/lib.rs` — start there and follow the imports.

---

## Development Workflow

### Repository Structure

```
crates/                     # Rust source code
├── neurograph-core/        # Core engine — this is where most work happens
│   └── src/
│       ├── lib.rs           # Public API (NeuroGraph struct)
│       ├── drivers/         # Storage backends (memory, sled, neo4j)
│       ├── graph/           # Data model (Entity, Relationship, Community)
│       ├── ingestion/       # Extraction & dedup pipeline
│       ├── retrieval/       # Hybrid search (semantic + BM25 + graph walk)
│       ├── temporal/        # Bi-temporal engine, forgetting
│       ├── community/       # Louvain, Leiden, summarizer
│       ├── llm/             # LLM client abstraction
│       ├── embedders/       # Embedding providers
│       ├── engine/          # Query routing & context assembly
│       └── utils/           # Concurrency, cost tracking
dashboard/                  # React + TypeScript + Vite (AntV G6)
docker/                     # Docker build files
deploy/                     # Docker Compose & deployment configs
benchmarks/                 # Performance benchmark suite
examples/                   # Usage examples
docs/                       # Architecture & deep-dive documentation
```

### Code Standards

**Rust:**

| Check | Command | Must Pass |
|-------|---------|-----------|
| Format | `cargo fmt --all -- --check` | ✅ |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| Tests | `cargo test --workspace` | ✅ |
| Audit | `cargo audit` | ⚠️ (advisory) |

- **MSRV:** Minimum Supported Rust Version is **1.82**
- **Doc comments:** All public functions must have `///` doc comments with examples where appropriate
- **Error handling:** Use `thiserror` for domain errors, `anyhow` only in examples/tests
- **Async:** All I/O operations must be async (`#[async_trait]`)
- **Naming:** Follow Rust API guidelines — `snake_case` for functions, `CamelCase` for types

**TypeScript/React (Dashboard):**

| Check | Command | Must Pass |
|-------|---------|-----------|
| Format | `npx prettier --check src/` | ✅ |
| Lint | `npm run lint` | ✅ |
| Build | `npm run build` | ✅ |

- Use TypeScript strict mode
- Functional components with hooks (no class components)
- Follow the existing component structure in `dashboard/src/components/`

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Examples:**

```
feat(core): add temporal diff API
fix(dashboard): fix minimap rendering on zoom
docs: update architecture diagram
perf(search): optimize BM25 scoring by 3x
refactor(drivers): extract shared trait methods
test(community): add Leiden convergence tests
chore(deps): update tokio to 1.40
ci: add WASM build step to release pipeline
```

**Prefix types:**

| Type | When to Use |
|------|------------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `perf` | Performance improvement |
| `refactor` | Code change that neither fixes nor adds |
| `test` | Adding or updating tests |
| `chore` | Build, CI, dependency updates |
| `ci` | CI/CD pipeline changes |
| `style` | Formatting, whitespace changes |

**Scope:** Use the module name (e.g., `core`, `dashboard`, `temporal`, `community`, `retrieval`, `drivers`).

---

## Pull Requests

### PR Checklist

Before submitting, please ensure:

- [ ] All CI checks pass (`cargo test`, `cargo clippy`, `cargo fmt`)
- [ ] New code has tests and doc comments
- [ ] Breaking API changes are documented in the PR description
- [ ] The PR is focused — one feature or fix per PR
- [ ] Commit messages follow Conventional Commits format
- [ ] You've updated relevant documentation (README, docs/, CHANGELOG)
- [ ] You've run the full test suite locally

### What Gets Reviewed

Our review focuses on:

| Category | What We Check |
|----------|--------------|
| **Correctness** | Does the code do what it claims? Are edge cases handled? |
| **Tests** | Are there sufficient tests? Do they cover failure modes? |
| **Performance** | Any hot path regressions? Unnecessary allocations? |
| **API design** | Is the public API consistent with existing patterns? |
| **Documentation** | Are public items documented? Are examples correct? |
| **Security** | Any user input not sanitized? Any unsafe blocks? |

### Review SLA

| PR Type | Target Response Time |
|---------|---------------------|
| Bug fix | 24-48 hours |
| Documentation | 24-48 hours |
| New feature | 3-5 business days |
| Refactoring | 3-5 business days |
| Architecture change | 1-2 weeks |

---

## Reporting Issues

### Bug Reports

Use the [Bug Report template](https://github.com/neurographai/neurograph/issues/new?template=bug_report.yml).

Include:
1. **Summary** — One-sentence description
2. **Steps to reproduce** — Minimal code or commands
3. **Expected behavior** — What should happen
4. **Actual behavior** — What actually happens
5. **Environment** — OS, Rust version, feature flags

### Feature Requests

Use the [Feature Request template](https://github.com/neurographai/neurograph/issues/new?template=feature_request.yml).

Include:
1. **Problem** — What problem are you trying to solve?
2. **Proposed solution** — How do you envision it working?
3. **Alternatives considered** — What else did you consider?
4. **Use case** — Real-world scenario where this would help

### Security Vulnerabilities

**DO NOT open a public issue.** See [SECURITY.md](SECURITY.md) for instructions on responsible disclosure.

---

## Good First Issues

Look for issues labeled [`good first issue`](https://github.com/neurographai/neurograph/labels/good%20first%20issue). These are:

- Well-defined scope
- Clear acceptance criteria
- Don't require deep knowledge of the codebase
- Have a maintainer available to help

Examples of good first contributions:
- Adding a new date format to `TemporalManager::parse_date()`
- Writing tests for edge cases in existing code
- Improving error messages with more context
- Adding doc examples to public functions
- Fixing Clippy warnings
- Documentation improvements

---

## Architecture & Technical References

| Document | Content |
|----------|---------|
| [Architecture](docs/architecture.md) | System design, data flow, module overview |
| [Temporal Engine](docs/temporal.md) | Bi-temporal model, forgetting, branching |
| [Community Detection](docs/community.md) | Louvain/Leiden algorithms, summarization |
| [Developer Guide](DEVELOPING.md) | Build, test, debug, profile |
| [Security Policy](SECURITY.md) | Vulnerability reporting, security measures |
| [Changelog](CHANGELOG.md) | Version history |

**External references:**
- [Graphiti (Zep)](https://github.com/getzep/graphiti) — Bi-temporal model inspiration
- [GraphRAG (Microsoft)](https://github.com/microsoft/graphrag) — Community detection + map-reduce
- [Mem0](https://github.com/mem0ai/mem0) — Memory optimization patterns
- [AntV G6](https://g6.antv.antgroup.com/) — Graph visualization library

---

## Community

- [**GitHub Discussions**](https://github.com/neurographai/neurograph/discussions) — Questions, ideas, show-and-tell
  - Q&A: Technical questions
  - Ideas: Feature proposals
  - Show and Tell: Share what you've built with NeuroGraph
- [**Issues**](https://github.com/neurographai/neurograph/issues) — Bug reports and tracked feature requests

---

## Recognition

Contributors are recognized in:
- The [CHANGELOG.md](CHANGELOG.md) for their specific contributions
- The GitHub contributor graph
- The project README (for significant contributions)
- Release notes crediting specific contributors

---

## License

By contributing, you agree that your contributions will be licensed under the [Apache-2.0 License](LICENSE).

---

Thank you for helping make NeuroGraph better! 🙂❤️
