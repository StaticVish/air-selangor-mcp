# syntax=docker/dockerfile:1

# -------------------------------------------------------------------
# Stage 1: Build static Rust binary with musl
# -------------------------------------------------------------------
FROM rust:alpine AS builder

# Install build dependencies including protoc and musl tools
RUN apk add --no-cache \
    musl-dev \
    protobuf-dev \
    protoc \
    build-base

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ ./proto/
COPY src/ ./src/

# Build optimized release binary
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked && \
    cp target/release/air-selangor-mcp /app/air-selangor-mcp && \
    strip /app/air-selangor-mcp

# -------------------------------------------------------------------
# Stage 2: Minimal Runtime (< 25 MB total)
# -------------------------------------------------------------------
FROM alpine:3.21 AS runner

LABEL org.opencontainers.image.title="air-selangor-mcp" \
      org.opencontainers.image.description="Statically compiled Model Context Protocol (MCP) server for Air Selangor utilities via Steel.dev and Protobuf 3" \
      org.opencontainers.image.authors="Visveswaran Vaidyanathan" \
      org.opencontainers.image.licenses="SEE LICENSE IN LICENSE" \
      org.opencontainers.image.source="https://github.com/StaticVish/air-selangor-mcp"

# Install SSL root certificates and timezone data
RUN apk add --no-cache ca-certificates tzdata

# Create non-root unprivileged service account
RUN addgroup -g 1000 mcp && \
    adduser -u 1000 -G mcp -s /bin/sh -D mcp

WORKDIR /app

# Set up runtime directories with ownership
RUN mkdir -p /app/.session /app/downloads && \
    chown -R mcp:mcp /app

# Copy compiled binary and proto schemas
COPY --from=builder --chown=mcp:mcp /app/air-selangor-mcp /app/air-selangor-mcp
COPY --from=builder --chown=mcp:mcp /app/proto /app/proto
COPY --chown=mcp:mcp LICENSE README.md /app/

# Switch to non-root user
USER mcp

# Set default downloads directory
ENV DOWNLOADS_DIR=/app/downloads

# Execution entrypoint
ENTRYPOINT ["/app/air-selangor-mcp"]
