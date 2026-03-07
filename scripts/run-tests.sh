#!/usr/bin/env bash
# Run all tests in an isolated environment.
# This spins up a separate Docker Compose stack (postgres + backend + frontend)
# on different ports from dev, runs backend unit/integration tests and
# frontend E2E tests, then tears everything down.
#
# Usage: ./scripts/run-tests.sh [--keep]
#   --keep    Don't tear down the test stack after running (for debugging)

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE="docker compose -f docker-compose.test.yml -p swarmcrest-test"
KEEP_STACK=false
EXIT_CODE=0

for arg in "$@"; do
    case "$arg" in
        --keep) KEEP_STACK=true ;;
    esac
done

cleanup() {
    if [ "$KEEP_STACK" = false ]; then
        echo ""
        echo "=== Tearing down test stack ==="
        cd "$PROJECT_ROOT"
        $COMPOSE down -v --remove-orphans 2>/dev/null || true
    else
        echo ""
        echo "=== Test stack left running (--keep). Tear down with: ==="
        echo "    docker compose -f docker-compose.test.yml -p swarmcrest-test down -v"
    fi
}
trap cleanup EXIT

cd "$PROJECT_ROOT"

echo "=== Starting isolated test stack ==="
$COMPOSE up --build -d

echo ""
echo "=== Waiting for backend to be ready ==="
for i in $(seq 1 120); do
    if curl -sf http://localhost:3100/health > /dev/null 2>&1; then
        echo "Backend ready."
        break
    fi
    if [ "$i" -eq 120 ]; then
        echo "ERROR: Backend did not become ready within 120s"
        echo "--- Backend logs ---"
        $COMPOSE logs test-backend --tail 30
        exit 1
    fi
    sleep 1
done

echo ""
echo "=== Waiting for frontend to be ready ==="
for i in $(seq 1 60); do
    if curl -sf http://localhost:5174/ > /dev/null 2>&1; then
        echo "Frontend ready."
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "ERROR: Frontend did not become ready within 60s"
        echo "--- Frontend logs ---"
        $COMPOSE logs test-frontend --tail 30
        exit 1
    fi
    sleep 1
done

echo ""
echo "=========================================="
echo "  Running backend tests"
echo "=========================================="
if $COMPOSE exec test-backend cargo test 2>&1; then
    echo "Backend tests: PASSED"
else
    echo "Backend tests: FAILED"
    EXIT_CODE=1
fi

echo ""
echo "=========================================="
echo "  Running E2E tests"
echo "=========================================="
cd "$PROJECT_ROOT/frontend"
if PLAYWRIGHT_BASE_URL=http://localhost:5174 npx playwright test 2>&1; then
    echo "E2E tests: PASSED"
else
    echo "E2E tests: FAILED"
    EXIT_CODE=1
fi

echo ""
if [ "$EXIT_CODE" -eq 0 ]; then
    echo "=========================================="
    echo "  ALL TESTS PASSED"
    echo "=========================================="
else
    echo "=========================================="
    echo "  SOME TESTS FAILED"
    echo "=========================================="
fi

exit $EXIT_CODE
