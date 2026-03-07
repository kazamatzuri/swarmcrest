# SwarmCrest Development Task Runner
# Use `just <recipe>` to run. Docker recipes are the default workflow;
# prefix with `local-` to run directly on the host.
#
# Cross-platform: works on Windows, macOS, and Linux.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# ---------- Docker Compose (default workflow) ----------

# Start all services in development mode
dev: up

# Start Docker Compose services (detached)
up:
    docker compose up --build -d

# Stop Docker Compose services
down:
    docker compose down

# Stop services and remove volumes (clean slate)
down-clean:
    docker compose down -v

# Rebuild containers from scratch (no cache)
rebuild:
    docker compose build --no-cache
    docker compose up -d

# View logs (follow mode)
logs:
    docker compose logs -f

# View backend logs only
logs-backend:
    docker compose logs -f backend

# View frontend logs only
logs-frontend:
    docker compose logs -f frontend

# Run backend tests inside the container
test:
    docker compose exec backend cargo test

# Run backend tests with output
test-verbose:
    docker compose exec backend cargo test -- --nocapture

# Validate original bot compatibility inside the container
validate-bots:
    docker compose exec backend cargo test -- original_bots --nocapture

# Run clippy checks inside the container
check:
    docker compose exec backend cargo clippy -- -D warnings

# Format backend code inside the container
fmt:
    docker compose exec backend cargo fmt

# Open a shell in the backend container
shell-backend:
    docker compose exec backend bash

# Open a shell in the frontend container
shell-frontend:
    docker compose exec frontend bash

# Run frontend lint inside the container
lint-frontend:
    docker compose exec frontend npm run lint

# Production build of frontend inside the container
build-frontend:
    docker compose exec frontend npm run build

# Production build of backend inside the container
build-backend:
    docker compose exec backend cargo build --release

# Build both services for production
build: build-backend build-frontend

# Show running containers and their status
status:
    docker compose ps

# ---------- Isolated test environment ----------

# Run all tests (backend + E2E) in an isolated Docker stack
test-all:
    ./scripts/run-tests.sh

# Run all tests but keep the test stack running for debugging
test-all-keep:
    ./scripts/run-tests.sh --keep

# Tear down the isolated test stack
test-down:
    docker compose -f docker-compose.test.yml -p swarmcrest-test down -v

# View test stack logs
test-logs:
    docker compose -f docker-compose.test.yml -p swarmcrest-test logs -f

# ---------- Production deployment ----------

# Deploy production stack (requires .env file with secrets)
prod-up:
    docker compose -f docker-compose.prod.yml up --build -d

# Stop production stack
prod-down:
    docker compose -f docker-compose.prod.yml down

# View production logs
prod-logs:
    docker compose -f docker-compose.prod.yml logs -f

# View production backend logs only
prod-logs-backend:
    docker compose -f docker-compose.prod.yml logs -f backend

# Production status
prod-status:
    docker compose -f docker-compose.prod.yml ps

# Run database backup
prod-backup:
    ./scripts/backup-db.sh

# Deploy with monitoring (Prometheus + Grafana)
prod-up-monitoring:
    docker compose -f docker-compose.prod.yml --profile monitoring up --build -d

# ---------- Production image & local play ----------

# Build the production Docker image
build-image:
    docker build -t swarmcrest:latest .

# Run locally with the production image (local mode, no auth)
local-run:
    docker compose -f docker-compose.local.yml up --build -d

# Stop the local instance
local-stop:
    docker compose -f docker-compose.local.yml down

# Stop local instance and remove data volume
local-clean:
    docker compose -f docker-compose.local.yml down -v

# View local instance logs
local-logs:
    docker compose -f docker-compose.local.yml logs -f

# ---------- Local development (no Docker) ----------
# Uses --manifest-path and --prefix to avoid shell-specific `cd &&` patterns.

# Start backend server locally
local-dev-backend:
    cargo run --manifest-path backend/Cargo.toml

# Start frontend dev server locally
local-dev-frontend:
    npm --prefix frontend run dev

# Run backend tests locally
local-test-backend:
    cargo test --manifest-path backend/Cargo.toml

# Run frontend tests locally
local-test-frontend:
    npm --prefix frontend test

# Run all local tests
local-test: local-test-backend

# Build backend locally (release)
local-build-backend:
    cargo build --manifest-path backend/Cargo.toml --release

# Build frontend locally (production)
local-build-frontend:
    npm --prefix frontend run build

# Check code locally
local-check:
    cargo clippy --manifest-path backend/Cargo.toml -- -D warnings

# Format code locally
local-fmt:
    cargo fmt --manifest-path backend/Cargo.toml

# Validate bot compatibility locally
local-validate-bots:
    cargo test --manifest-path backend/Cargo.toml -- original_bots --nocapture

# Install frontend dependencies locally
local-install-frontend:
    npm --prefix frontend install

# Run frontend e2e tests locally (requires backend + frontend running)
local-test-e2e:
    npm --prefix frontend run test:e2e

# Run frontend e2e tests with UI
local-test-e2e-ui:
    npm --prefix frontend run test:e2e:ui

# Run all local tests (backend unit + frontend e2e)
local-test-all: local-test-backend local-test-e2e
