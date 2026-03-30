# Contributing to NeuroGraph

Thank you for considering contributing to NeuroGraph! This guide will help you get started.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up the development environment (see [DEVELOPING.md](DEVELOPING.md))
4. Create a new branch for your feature or bugfix

## Development Workflow

### Repository Structure

- `crates/` — Rust source code (core engine, CLI, server, WASM, Python bindings)
- `dashboard/` — React + TypeScript frontend (AntV G6 visualization)
- `docker/` — Docker build files
- `deploy/` — Docker Compose and deployment configs
- `benchmarks/` — Performance benchmark suite
- `examples/` — Usage examples

### Code Standards

**Rust:**
- Format with `cargo fmt --all`
- Lint with `cargo clippy --workspace --all-targets -- -D warnings`
- Run tests with `cargo test --workspace`
- Minimum supported Rust version (MSRV): 1.82

**TypeScript/React (Dashboard):**
- Format with Prettier (`.prettierrc` config)
- Lint with ESLint
- Build check with `npm run build`

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(core): add temporal diff API
fix(dashboard): fix minimap rendering on zoom
docs: update architecture diagram
perf(search): optimize BM25 scoring
chore(deps): update tokio to 1.40
```

Prefix types: `feat`, `fix`, `docs`, `perf`, `refactor`, `test`, `chore`, `ci`, `style`

## Pull Requests

1. **Keep it focused**: One PR per feature or bug fix
2. **Describe your changes**: Fill out the PR template with context and rationale
3. **Pass checks**: Ensure all CI checks pass before requesting review
4. **Add tests**: New features and bug fixes should include tests
5. **Update docs**: Add documentation for any new public APIs

### What Gets Reviewed

- Code correctness and test coverage
- Performance implications (especially for hot paths)
- API design consistency
- Documentation quality

## Reporting Issues

- Use the **Bug Report** template for bugs
- Use the **Feature Request** template for new ideas
- Search existing issues before opening a new one

## Community

- [GitHub Discussions](https://github.com/neurographai/neurograph/discussions) for questions and ideas
- Issues for bug reports and feature requests

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.
