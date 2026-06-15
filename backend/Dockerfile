# syntax=docker/dockerfile:1.7

# Multi-stage build:
#   1. chef     — cargo-chef installed once, reused by planner & builder
#   2. planner  — produces recipe.json describing the dep tree
#   3. builder  — cooks deps from recipe (cached layer), then builds the app
#   4. runtime  — distroless, nonroot, ~25 MB, glibc + ca-certs only

# ─── Stage 1: chef base ───────────────────────────────────────────────
# Rust >= 1.88 is required by some transitive deps (time, uuid, idna_adapter
# which needs edition2024). Bump as needed when bumping the toolchain.
FROM rust:1.90-slim-bookworm AS chef
WORKDIR /app

# ─── Corporate CA injection (for SSL-inspecting proxies) ──────────────
# If `corporate-ca.pem` exists in the build context (even empty) it gets
# folded into the system trust store. `cargo` is told to use that store
# via SSL_CERT_FILE, which short-circuits the bundled webpki-roots logic.
# Other devs without a corporate proxy can `touch corporate-ca.pem` and
# the build is a no-op.
COPY corporate-ca.pem /usr/local/share/ca-certificates/corporate-ca.crt
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends ca-certificates \
 && update-ca-certificates \
 && rm -rf /var/lib/apt/lists/*
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
    CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt

RUN cargo install cargo-chef --locked

# ─── Stage 2: planner ─────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ─── Stage 3: builder ─────────────────────────────────────────────────
FROM chef AS builder

# Build only dependencies. This layer is cached unless Cargo.toml / Cargo.lock
# changes — source edits don't bust it. `--locked` makes cargo fail fast if
# Cargo.lock is missing or out of sync, so we never silently re-resolve to
# different versions inside Docker.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --locked

# Now copy the real source and build the binary. Migrations are embedded into
# the binary at compile time via `sqlx::migrate!`, so the runtime image doesn't
# need the migrations/ folder.
COPY . .
RUN cargo build --release -p api --locked \
 && strip target/release/api

# ─── Stage 4: runtime ─────────────────────────────────────────────────
# distroless/cc-debian12: glibc + ca-certificates, no shell, no package manager.
# `nonroot` user is uid 65532.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder --chown=nonroot:nonroot /app/target/release/api /app/api

USER nonroot:nonroot
EXPOSE 8080

# /health is the liveness/readiness probe endpoint — wire it up in your
# orchestrator (k8s probes, ALB health check, etc).
ENTRYPOINT ["/app/api"]
