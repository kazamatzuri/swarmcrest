#!/usr/bin/env bash
# Automated PostgreSQL backup for SwarmCrest production.
#
# Usage:
#   ./scripts/backup-db.sh                  # manual run
#   Install as cron job (daily at 3 AM, retain 7 days):
#     0 3 * * * /path/to/scripts/backup-db.sh
#
# Requires the production compose stack to be running.
# Backups are stored in ./backups/ relative to the project root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="${BACKUP_DIR:-$PROJECT_DIR/backups}"
RETAIN_DAYS="${RETAIN_DAYS:-7}"
COMPOSE_FILE="${COMPOSE_FILE:-$PROJECT_DIR/docker-compose.prod.yml}"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
BACKUP_FILE="$BACKUP_DIR/swarmcrest_${TIMESTAMP}.sql.gz"

mkdir -p "$BACKUP_DIR"

echo "[backup] Starting PostgreSQL backup at $(date)"

docker compose -f "$COMPOSE_FILE" exec -T postgres \
  pg_dump -U "${POSTGRES_USER:-swarmcrest}" "${POSTGRES_DB:-swarmcrest}" \
  | gzip > "$BACKUP_FILE"

BACKUP_SIZE="$(du -h "$BACKUP_FILE" | cut -f1)"
echo "[backup] Created $BACKUP_FILE ($BACKUP_SIZE)"

# Remove backups older than RETAIN_DAYS
DELETED=$(find "$BACKUP_DIR" -name "swarmcrest_*.sql.gz" -mtime +"$RETAIN_DAYS" -print -delete | wc -l)
if [ "$DELETED" -gt 0 ]; then
  echo "[backup] Removed $DELETED backup(s) older than $RETAIN_DAYS days"
fi

echo "[backup] Done at $(date)"
