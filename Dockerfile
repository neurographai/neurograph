# Base image for NeuroGraph API server (Sprint 4)
FROM rust:1.80-slim as builder

WORKDIR /usr/src/neurograph
COPY . .

# Build the workspace
RUN cargo build --release --workspace

# Runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
# We will copy the server binary here once Sprint 4 is implemented.
# COPY --from=builder /usr/src/neurograph/target/release/neurograph-server /app/

# Expose default API port
EXPOSE 8000

# Placeholder for the upcoming web server
CMD ["echo", "NeuroGraph Engine Container (API coming in Sprint 4)"]
