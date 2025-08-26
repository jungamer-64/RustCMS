## Multi-stage Dockerfile for Rust CMS
## Build-time args:
## - FEATURES: features string to pass to `cargo build` (e.g. "dev-tools,auth,database")
## - NO_DEFAULT_FEATURES: set to "true" to pass --no-default-features to cargo
## - BINARY: which binary from target/release to copy/run (default: cms-server)

ARG RUST_VERSION=latest
FROM rust:${RUST_VERSION} AS builder
ARG FEATURES=""
ARG NO_DEFAULT_FEATURES="false"
ARG BINARY="cms-server"

WORKDIR /app

# Install system deps commonly required by crates (OpenSSL, Postgres client libs, pkg-config)
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       build-essential \
       pkg-config \
       libssl-dev \
       libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Configure cargo paths so we can cache registry/git/target directories
ENV CARGO_HOME=/usr/local/cargo
ENV CARGO_TARGET_DIR=/usr/local/cargo/target
RUN mkdir -p ${CARGO_HOME} ${CARGO_TARGET_DIR} && chown -R root:root ${CARGO_HOME} ${CARGO_TARGET_DIR}

# Copy the full project early to ensure all manifest files (including Cargo.lock) are present
# This sacrifices one Docker layer optimality for robustness across CI environments where
# the build context or sparse checkouts can omit top-level files when COPY is targeted.
COPY . .

# Use BuildKit cache mounts to persist cargo registry/git/target between builds (faster incremental builds)
# Note: requires Docker BuildKit (DOCKER_BUILDKIT=1) to be enabled when building.
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-index,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/usr/local/cargo/target \
    cargo fetch --locked || true

# Build release with optional features
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
        --mount=type=cache,id=cargo-index,target=/usr/local/cargo/git \
        --mount=type=cache,id=cargo-target,target=/usr/local/cargo/target \
        if [ "$NO_DEFAULT_FEATURES" = "true" ]; then \
            if [ -n "$FEATURES" ]; then \
                # Try reproducible build with --locked, fall back to unlocked when lockfile is out-of-date
                sh -c "cargo build --release --no-default-features --features \"$FEATURES\" --locked || cargo build --release --no-default-features --features \"$FEATURES\""; \
            else \
                cargo build --release --no-default-features --locked || cargo build --release --no-default-features; \
            fi; \
        else \
            if [ -n "$FEATURES" ]; then \
                sh -c "cargo build --release --features \"$FEATURES\" --locked || cargo build --release --features \"$FEATURES\""; \
            else \
                cargo build --release --locked || cargo build --release; \
            fi; \
        fi

## Runtime stage
FROM debian:bookworm-slim AS runtime
ARG BINARY="admin_server"

# Minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and migrations (if any)
COPY --from=builder /app/target/release/${BINARY} /usr/local/bin/${BINARY}
COPY --from=builder /app/migrations ./migrations

# Create a non-root user and make app directory owned by it
RUN useradd -r -s /bin/false cms && chown -R cms:cms /app /usr/local/bin/${BINARY}
USER cms

# Default env placeholders (override at runtime)
ENV CONFIG_FILE=/app/config/default.toml
ENV CMS_ENVIRONMENT=production

# Expose default port
EXPOSE 3000

# Lightweight healthcheck; endpoint should exist in the app
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Default command: run selected binary
ENTRYPOINT ["/usr/local/bin/admin_server"]

