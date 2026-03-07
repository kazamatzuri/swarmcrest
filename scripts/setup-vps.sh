#!/usr/bin/env bash
# First-time VPS setup for SwarmCrest production deployment.
#
# Run on the VPS as root or with sudo:
#   curl -fsSL <raw-github-url> | bash
#   # or: scp scripts/setup-vps.sh user@vps: && ssh user@vps bash setup-vps.sh
#
# What this does:
#   1. Installs Docker + Docker Compose plugin
#   2. Creates a deploy user with Docker access
#   3. Sets up the project directory
#   4. Configures daily database backups via cron
#
# After running, you still need to:
#   - Add the deploy user's SSH key to GitHub secrets (VPS_SSH_KEY)
#   - Copy .env.example to ~/swarmcrest/.env and fill in secrets
#   - Set up DNS A record pointing to this server
#   - Log in to GHCR: docker login ghcr.io

set -euo pipefail

DEPLOY_USER="${DEPLOY_USER:-deploy}"
PROJECT_DIR="/home/$DEPLOY_USER/swarmcrest"

echo "[setup] Installing Docker..."
if ! command -v docker &>/dev/null; then
  curl -fsSL https://get.docker.com | sh
fi

echo "[setup] Creating deploy user: $DEPLOY_USER"
if ! id "$DEPLOY_USER" &>/dev/null; then
  useradd -m -s /bin/bash "$DEPLOY_USER"
  usermod -aG docker "$DEPLOY_USER"
  echo "[setup] Generate an SSH key for CI/CD:"
  echo "  sudo -u $DEPLOY_USER ssh-keygen -t ed25519 -f /home/$DEPLOY_USER/.ssh/id_ed25519 -N ''"
  echo "  Then add the public key to /home/$DEPLOY_USER/.ssh/authorized_keys"
else
  usermod -aG docker "$DEPLOY_USER"
  echo "[setup] User $DEPLOY_USER already exists, added to docker group"
fi

echo "[setup] Creating project directory..."
sudo -u "$DEPLOY_USER" mkdir -p "$PROJECT_DIR"

# Copy required compose and config files
for file in docker-compose.prod.yml Caddyfile .env.example prometheus.yml scripts/backup-db.sh; do
  if [ -f "$file" ]; then
    dir=$(dirname "$PROJECT_DIR/$file")
    sudo -u "$DEPLOY_USER" mkdir -p "$dir"
    cp "$file" "$PROJECT_DIR/$file"
    chown "$DEPLOY_USER:$DEPLOY_USER" "$PROJECT_DIR/$file"
  fi
done

# Copy grafana provisioning if present
if [ -d "grafana" ]; then
  cp -r grafana "$PROJECT_DIR/"
  chown -R "$DEPLOY_USER:$DEPLOY_USER" "$PROJECT_DIR/grafana"
fi

chmod +x "$PROJECT_DIR/scripts/backup-db.sh" 2>/dev/null || true

echo "[setup] Installing daily backup cron job..."
CRON_LINE="0 3 * * * $PROJECT_DIR/scripts/backup-db.sh >> /var/log/swarmcrest-backup.log 2>&1"
(sudo -u "$DEPLOY_USER" crontab -l 2>/dev/null | grep -v backup-db.sh; echo "$CRON_LINE") | sudo -u "$DEPLOY_USER" crontab -

echo ""
echo "[setup] Done! Next steps:"
echo "  1. cp $PROJECT_DIR/.env.example $PROJECT_DIR/.env"
echo "  2. Edit $PROJECT_DIR/.env with real secrets"
echo "  3. Log in to GHCR: sudo -u $DEPLOY_USER docker login ghcr.io"
echo "  4. Add GitHub secrets: VPS_HOST, VPS_USER=$DEPLOY_USER, VPS_SSH_KEY"
echo "  5. Push to master to trigger first deploy, or manually:"
echo "     cd $PROJECT_DIR && docker compose -f docker-compose.prod.yml pull && docker compose -f docker-compose.prod.yml up -d"
