# Multi-stage production Dockerfile for SwarmCrest Battle Arena
# Builds frontend and backend into a single minimal image.

# ── Stage 1: Build frontend ──────────────────────────────────────────
FROM node:22-bookworm-slim AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ── Stage 2: Build backend ───────────────────────────────────────────
FROM rust:1-bookworm AS backend-builder

# Install build dependencies for mlua (lua51 vendored)
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libreadline-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy files needed at compile time by include_str! macros
COPY orig_game/ orig_game/
COPY docs/ docs/

WORKDIR /app/backend

# Pre-fetch dependencies (layer caching)
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo 'fn main() { println!("placeholder"); }' > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build the actual backend
COPY backend/src/ src/
# Touch main.rs to ensure it rebuilds with real source
RUN touch src/main.rs
RUN cargo build --release

# ── Stage 3: Runtime ─────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the backend binary
COPY --from=backend-builder /app/backend/target/release/swarmcrest-backend ./swarmcrest-backend

# Copy frontend build output
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

# Copy map data
COPY data/maps ./data/maps

ENV MAPS_DIR=/app/data/maps
ENV STATIC_DIR=/app/frontend/dist
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000

ENTRYPOINT ["./swarmcrest-backend"]
