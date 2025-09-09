## syntax=docker/dockerfile:1.7
## Multi-stage Dockerfile (optimized) for Rust CMS
## Build args:
##  - RUST_VERSION: base rust image tag (e.g. 1.80-bookworm)
##  - FEATURES: cargo features list
##  - NO_DEFAULT_FEATURES: set "true" to disable default features
##  - BINARY: runtime binary to copy (default cms-server)
##  - BUILD_VARIANT: label/metadata hint (e.g. production, dev)

ARG RUST_VERSION=1.89.0-bookworm

# --- Planner stage (dependency graph) using cargo-chef ---
FROM rust:${RUST_VERSION} AS planner
ARG CHEF_VERSION="0.1.71"
WORKDIR /app
RUN cargo install cargo-chef --locked --version ${CHEF_VERSION}
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Copy feature-dependent build scripts or additional manifests if any (adjust as needed)
COPY migrations ./migrations
COPY templates ./templates
COPY config ./config
RUN cargo chef prepare --recipe-path recipe.json

# --- Builder stage ---
FROM rust:${RUST_VERSION} AS builder
ARG FEATURES=""
ARG NO_DEFAULT_FEATURES="false"
ARG BINARY="cms-server"          # primary runtime binary
ARG BUILD_VARIANT="unknown"
ARG BUILD_EXTRA_BINS=""           # space-separated additional bin names (optional)
ARG TARGET=""                     # e.g. x86_64-unknown-linux-gnu (empty => default toolchain target)
ARG USE_CHEF="true"               # set to false to disable cargo-chef cook optimization
ARG PARALLEL_JOBS=""             # override cargo build -j; empty:auto

WORKDIR /app

# System build dependencies (kept minimal). We install only what is required to compile.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
        --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
        apt-get update \
 && apt-get install -y --no-install-recommends \
            build-essential \
            pkg-config \
            binutils \
            libssl-dev \
            libpq-dev \
            nasm \
        && rm -rf /var/lib/apt/lists/*

# Cache dirs (shared across BuildKit mounts)
ENV CARGO_HOME=/usr/local/cargo \
    CARGO_TARGET_DIR=/usr/local/cargo/target \
    # Favor smaller code (already optimized via profile) and strip symbols (defensive)
    RUSTFLAGS="-C debuginfo=0 -C strip=symbols" \
    # Make git fetch deterministic and sometimes faster inside certain CI proxies
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    # Reproducible builds (override at build time if needed)
    SOURCE_DATE_EPOCH=1704067200

ARG VCS_REF="unknown"
ARG BUILD_DATE="unknown"

# Bring in recipe (always copied, can be ignored if USE_CHEF=false)
COPY --from=planner /app/recipe.json ./recipe.json

# If using cargo-chef, cook dependency layer (no source code yet for maximal cache reuse)
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
        --mount=type=cache,id=cargo-index,target=/usr/local/cargo/git,sharing=locked \
        --mount=type=cache,id=cargo-target,target=/usr/local/cargo/target \
        if [ "$USE_CHEF" = "true" ]; then \
            echo "[chef] Cooking dependency layer"; \
            feature_flags=""; \
            if [ "$NO_DEFAULT_FEATURES" = "true" ]; then feature_flags="--no-default-features"; fi; \
            if [ -n "$FEATURES" ]; then feature_flags="$feature_flags --features $FEATURES"; fi; \
            (cargo chef cook --release --recipe-path recipe.json $feature_flags --locked || \
             cargo chef cook --release --recipe-path recipe.json $feature_flags); \
        else \
            echo "[chef] Skipped (USE_CHEF=false)"; \
        fi

# Copy full source after dependency layer to minimize rebuilds
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY config ./config

## Build only the required binaries (fast incremental layer following dependency cook)
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
        --mount=type=cache,id=cargo-index,target=/usr/local/cargo/git,sharing=locked \
        --mount=type=cache,id=cargo-target,target=/usr/local/cargo/target \
        set -eux; \
        JOBS_ARG=""; if [ -n "$PARALLEL_JOBS" ]; then JOBS_ARG="-j $PARALLEL_JOBS"; fi; \
        BUILD_CMD_BASE="cargo build --release $JOBS_ARG"; \
        if [ -n "$TARGET" ]; then BUILD_CMD_BASE="$BUILD_CMD_BASE --target $TARGET"; fi; \
        feature_flags=""; \
        if [ "$NO_DEFAULT_FEATURES" = "true" ]; then feature_flags="--no-default-features"; fi; \
        if [ -n "$FEATURES" ]; then feature_flags="$feature_flags --features $FEATURES"; fi; \
        echo "[build] Primary bin: $BINARY  Extra bins: ${BUILD_EXTRA_BINS}"; \
        sh -c "$BUILD_CMD_BASE --bin $BINARY $feature_flags --locked" || \
            sh -c "$BUILD_CMD_BASE --bin $BINARY $feature_flags"; \
            if [ -n "$BUILD_EXTRA_BINS" ]; then \
                for extra in $BUILD_EXTRA_BINS; do \
                    echo "[build] Extra bin: $extra"; \
                    sh -c "$BUILD_CMD_BASE --bin $extra $feature_flags --locked" || \
                        sh -c "$BUILD_CMD_BASE --bin $extra $feature_flags"; \
                done; \
            fi; \
            if [ "${DEBUG_BUILD_LIST:-0}" = "1" ]; then ls -lah /usr/local/cargo/target/release || true; fi; \
            # Strip binaries to reduce size if strip is available
            if command -v strip >/dev/null 2>&1; then \
                TARGET_DIR_BASE=/usr/local/cargo/target; \
                if [ -n "$TARGET" ]; then \
                    CAND_DIR="$TARGET_DIR_BASE/$TARGET/release"; \
                else \
                    CAND_DIR="$TARGET_DIR_BASE/release"; \
                fi; \
                echo "[strip] Stripping binaries in $CAND_DIR"; \
                for bin in ${BINARY} $BUILD_EXTRA_BINS; do \
                    if [ -f "$CAND_DIR/$bin" ]; then \
                        strip --strip-all "$CAND_DIR/$bin" || true; \
                    fi; \
                done; \
            fi

LABEL stage=builder \
    org.opencontainers.image.title="Rust CMS (${BUILD_VARIANT})" \
    org.opencontainers.image.source="${CI_REPO_URL:-unknown}" \
    org.opencontainers.image.revision="${VCS_REF}" \
    org.opencontainers.image.created="${BUILD_DATE}" \
    org.opencontainers.image.vendor="RustCMS" \
    org.opencontainers.image.licenses="MIT"

### Runtime image
FROM debian:bookworm-slim AS runtime
ARG BINARY="admin_server"
ARG BUILD_VARIANT="unknown"
ARG BUILD_EXTRA_BINS=""
ARG TARGET=""
ARG APP_UID=10001
ARG APP_GID=10001
ARG VCS_REF="unknown"
ARG BUILD_DATE="unknown"

# Install only runtime essentials (curl kept for healthcheck; remove if not needed) with apt cache mounts
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
        --mount=type=cache,target=/var/lib/apt/lists,sharing=locked \
        apt-get update && apt-get install -y --no-install-recommends \
            ca-certificates \
            libssl3 \
            curl \
        && rm -rf /var/lib/apt/lists/*

ENV TZ=UTC
WORKDIR /app

# Create non-root user/group early so COPY --chown can set ownership directly
RUN set -eux; \
        groupadd --system -g ${APP_GID} cms || true; \
        useradd  --system -g ${APP_GID} -u ${APP_UID} --home /app --shell /usr/sbin/nologin cms || true

# Determine target triple (optional informational file)
RUN set -eux; if [ -n "$TARGET" ]; then echo $TARGET > /tmp/target-triple; else echo "" > /tmp/target-triple; fi
ENV TARGET_TRIPLE_FILE=/tmp/target-triple

# Copy primary binary & assets with ownership set directly
COPY --from=builder --chown=cms:cms /usr/local/cargo/target/release/${BINARY} /usr/local/bin/${BINARY}
COPY --from=builder --chown=cms:cms /app/migrations ./migrations
COPY --from=builder --chown=cms:cms /app/config ./config
COPY --from=builder --chown=cms:cms /app/templates ./templates

# Copy extra binaries (if any) preserving ownership
RUN set -eux; \
    if [ -n "$BUILD_EXTRA_BINS" ]; then \
        for extra in $BUILD_EXTRA_BINS; do \
            if [ -f "/usr/local/cargo/target/release/$extra" ]; then \
                cp "/usr/local/cargo/target/release/$extra" "/usr/local/bin/$extra"; \
                chown cms:cms "/usr/local/bin/$extra"; \
            fi; \
        done; \
    fi || true

USER cms

ENV CONFIG_FILE=/app/config/default.toml \
        CMS_ENVIRONMENT=production

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:3000/health || exit 1

LABEL org.opencontainers.image.title="Rust CMS (${BUILD_VARIANT})" \
    org.opencontainers.image.description="Multi-binary Rust CMS container (variant: ${BUILD_VARIANT})" \
    org.opencontainers.image.source="${CI_REPO_URL:-unknown}" \
    org.opencontainers.image.revision="${VCS_REF}" \
    org.opencontainers.image.created="${BUILD_DATE}" \
    org.opencontainers.image.vendor="RustCMS" \
    org.opencontainers.image.licenses="MIT"

# Use shell form so $BINARY expands correctly (JSON form would not expand env vars)
ENTRYPOINT /usr/local/bin/${BINARY}

## Optional test stage (invoked explicitly: docker build --target test ...)
FROM builder AS test
ARG RUN_TESTS="false"
ARG TEST_FEATURES=""
ARG NO_DEFAULT_FEATURES="false"
ARG PARALLEL_JOBS=""
RUN set -eux; \
        if [ "$RUN_TESTS" = "true" ]; then \
            feature_flags=""; \
            if [ "$NO_DEFAULT_FEATURES" = "true" ]; then feature_flags="--no-default-features"; fi; \
            if [ -n "$TEST_FEATURES" ]; then feature_flags="$feature_flags --features $TEST_FEATURES"; fi; \
            JOBS_ARG=""; if [ -n "$PARALLEL_JOBS" ]; then JOBS_ARG="-j $PARALLEL_JOBS"; fi; \
            echo "[tests] Running cargo test $feature_flags"; \
            (cargo test $JOBS_ARG --workspace --all-targets $feature_flags --locked || \
             cargo test $JOBS_ARG --workspace --all-targets $feature_flags); \
        else \
            echo "[tests] Skipped (RUN_TESTS=false)"; \
        fi