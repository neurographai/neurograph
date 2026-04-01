# ============================================================
# NEUROGRAPH PRODUCTION DOCKERFILE
# Multi-stage build: Rust compile > Dashboard build > Final slim image
# ============================================================

# Stage 1: Build Rust binaries
FROM rust:1.82-bookworm AS rust-builder

RUN apt-get update && apt-get install -y \
    cmake \
    protobuf-compiler \
    libssl-dev \
    pkg-config \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

ARG PROFILE=release
ARG FEATURES=default

WORKDIR /build

# Cache dependency builds by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --profile ${PROFILE} --features ${FEATURES} --bin neurograph-cli --bin neurograph-server

# Stage 2: Build Dashboard (React)
FROM node:22-bookworm-slim AS dashboard-builder

WORKDIR /build

COPY dashboard/package.json dashboard/package-lock.json* ./
RUN npm ci --production=false 2>/dev/null || npm install

COPY dashboard/ ./

ENV NODE_ENV=production
RUN npm run build

# Stage 3: Final production image
FROM debian:bookworm-slim AS production

LABEL org.opencontainers.image.source="https://github.com/neurographai/neurograph"
LABEL org.opencontainers.image.description="NeuroGraph: Rust-powered temporal knowledge graph engine"
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.vendor="NeuroGraph"

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r neurograph \
    && useradd -r -g neurograph -d /app -s /sbin/nologin neurograph

WORKDIR /app

# Copy rust binaries
COPY --from=rust-builder /build/target/release/neurograph-cli /usr/local/bin/neurograph
COPY --from=rust-builder /build/target/release/neurograph-server /usr/local/bin/neurograph-server

# Copy dashboard build
COPY --from=dashboard-builder /build/dist /app/dashboard/

# Extract embedded entrypoint directly to bin
RUN echo '#!/bin/bash\nexec "$@"' > /usr/local/bin/docker-entrypoint.sh \
    && chmod +x /usr/local/bin/docker-entrypoint.sh

# Create data directories
RUN mkdir -p /app/data /app/logs /app/config \
    && chown -R neurograph:neurograph /app

ENV NEUROGRAPH_HOST=0.0.0.0 \
    NEUROGRAPH_API_PORT=8000 \
    NEUROGRAPH_DASHBOARD_PORT=3000 \
    NEUROGRAPH_DATA_DIR=/app/data \
    NEUROGRAPH_LOG_LEVEL=info \
    NEUROGRAPH_STORAGE=embedded \
    RUST_LOG=neurograph=info

EXPOSE 8000 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

VOLUME ["/app/data", "/app/config"]

USER neurograph

ENTRYPOINT ["tini", "--", "docker-entrypoint.sh"]
CMD ["neurograph-server"]
