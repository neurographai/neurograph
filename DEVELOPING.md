# Developer Guide

Welcome to the internal documentation for developing NeuroGraph! 🛠️

## Prerequisites

- **Rust**: Version 1.80+ (`rustup default stable`)
- **Node.js**: Version 18+ for the dashboard
- **wasm-pack**: For compiling the WASM graph layouts (`cargo install wasm-pack`)
- **Docker**: For running integration tests

## Architecture Overview

The repository is structured as a monolithic workspace:

- `crates/neurograph-core`: The main Rust engine containing the bi-temporal engine, graph traversal logic, community detection, and database bindings.
- `crates/neurograph-wasm`: Contains WebAssembly bindings targeting graph physics and computations we inject into the frontend.
- `dashboard/`: A React + TypeScript Vite application designed around AntV G6 for massive graph rendering.

## Running the Application Locally

1. **Build and Run Backend (Rust)**:
    ```bash
    cd crates/neurograph-core
    cargo run --release -- --serve
    ```

2. **Run Frontend (Web)**:
    ```bash
    cd dashboard
    npm install
    npm run dev
    ```

## Testing

```bash
# Run Rust unit tests
cargo test --workspace

# Run Integration tests (Requires Docker for DB backends like neo4j/redis if applicable)
cargo test --test integration_tests
```

## Creating a new release

Releases are automated via GitHub actions when tags are pushed:
```bash
git tag v0.1.0
git push origin v0.1.0
```
