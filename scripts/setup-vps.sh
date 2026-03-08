#!/usr/bin/env bash
# VPS setup for SwarmCrest production deployment.
#
# Prerequisites (already done):
#   - Docker installed, deploy user created with Docker access
#   - SSH keys configured (CI key + GitHub deploy key)
#   - Repo cloned to ~/swarmcrest
#
# Run on the VPS as root:
#   bash ~/swarmcrest/scripts/setup-vps.sh
#
# After running, you still need to:
#   - Add GitHub repo secrets (printed at the end)
#   - Fill in OAuth credentials in .env (when ready)

set -euo pipefail

DEPLOY_USER="${DEPLOY_USER:-swarmcrest}"
PROJECT_DIR="/home/$DEPLOY_USER/swarmcrest"
DOMAIN="${DOMAIN:-swarmcrest.submerged-intelligence.de}"
GITHUB_REPO="${GITHUB_REPO:-kazamatzuri/swarmcrest}"

info()  { echo -e "\n\033[1;34m[setup]\033[0m $*"; }
warn()  { echo -e "\033[1;33m[warn]\033[0m $*"; }
error() { echo -e "\033[1;31m[error]\033[0m $*" >&2; exit 1; }

[[ $EUID -eq 0 ]] || error "This script must be run as root."
[[ -d "$PROJECT_DIR/.git" ]] || error "Repo not found at $PROJECT_DIR — clone it first."

# ── 1. Create skeleton .env ──────────────────────────────────────────

ENV_FILE="$PROJECT_DIR/.env"
if [[ ! -f "$ENV_FILE" ]]; then
  info "Creating .env with generated secrets..."
  JWT_SECRET=$(openssl rand -base64 32)
  PG_PASSWORD=$(openssl rand -base64 24)

  cat > "$ENV_FILE" << EOF
# SwarmCrest production environment — KEEP THIS SECRET
DOMAIN=$DOMAIN
POSTGRES_USER=swarmcrest
POSTGRES_PASSWORD=$PG_PASSWORD
POSTGRES_DB=swarmcrest
JWT_SECRET=$JWT_SECRET
RUST_LOG=info

# OAuth (fill in when ready)
GITHUB_CLIENT_ID=
GITHUB_CLIENT_SECRET=
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=

# Monitoring (optional)
METRICS_TOKEN=
GRAFANA_ADMIN_PASSWORD=
EOF
  chmod 600 "$ENV_FILE"
  chown "$DEPLOY_USER:$DEPLOY_USER" "$ENV_FILE"
  info "Generated random JWT_SECRET and POSTGRES_PASSWORD."
else
  info ".env already exists, not touching it."
fi

# ── 2. Nginx + TLS ───────────────────────────────────────────────────

for pkg in nginx certbot python3-certbot-nginx; do
  if ! dpkg -s "$pkg" &>/dev/null 2>&1; then
    info "Installing $pkg..."
    apt-get update -qq && apt-get install -y -qq "$pkg"
  fi
done

NGINX_CONF="$PROJECT_DIR/nginx/swarmcrest.conf"
if [[ -f "$NGINX_CONF" ]]; then
  info "Installing nginx site config..."

  # Create webroot for ACME challenges
  mkdir -p /var/www/certbot

  cp "$NGINX_CONF" /etc/nginx/sites-available/swarmcrest
  ln -sf /etc/nginx/sites-available/swarmcrest /etc/nginx/sites-enabled/swarmcrest

  if nginx -t 2>/dev/null; then
    systemctl reload nginx
    info "Nginx config installed and loaded."

    info "Requesting TLS certificate via webroot..."
    certbot certonly --webroot -w /var/www/certbot -d "$DOMAIN" \
      --non-interactive --agree-tos --register-unsafely-without-email || \
      warn "Certbot failed — check DNS A record points to this server."

    # Let certbot's --nginx plugin wire the cert into the config
    if [[ -f "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" ]]; then
      certbot install --nginx -d "$DOMAIN" --non-interactive || \
        warn "Certbot install failed — you may need to run: certbot --nginx -d $DOMAIN"
    fi
  else
    warn "Nginx config test failed — check /etc/nginx/sites-available/swarmcrest"
  fi
else
  error "nginx/swarmcrest.conf not found in repo."
fi

# ── 3. GHCR login ────────────────────────────────────────────────────

info "Logging into GHCR (GitHub Container Registry)..."
echo "  You need a GitHub PAT with read:packages scope."
echo "  Create one at: https://github.com/settings/tokens"
echo ""
read -r -p "GitHub username: " GHCR_USER
read -r -s -p "GitHub PAT: " GHCR_TOKEN
echo ""
echo "$GHCR_TOKEN" | sudo -u "$DEPLOY_USER" docker login ghcr.io -u "$GHCR_USER" --password-stdin

# ── 4. Backup cron ───────────────────────────────────────────────────

BACKUP_DIR="/home/$DEPLOY_USER/backups"
sudo -u "$DEPLOY_USER" mkdir -p "$BACKUP_DIR"

BACKUP_SCRIPT="$PROJECT_DIR/scripts/backup-db.sh"
if [[ -f "$BACKUP_SCRIPT" ]]; then
  chmod +x "$BACKUP_SCRIPT"
  CRON_LINE="0 3 * * * $BACKUP_SCRIPT >> /var/log/swarmcrest-backup.log 2>&1"
  (sudo -u "$DEPLOY_USER" crontab -l 2>/dev/null | grep -v backup-db.sh; echo "$CRON_LINE") \
    | sudo -u "$DEPLOY_USER" crontab -
  info "Daily backup cron installed (03:00)."
fi

# ── Done ──────────────────────────────────────────────────────────────

CI_KEY="/home/$DEPLOY_USER/.ssh/ci_ed25519"

echo ""
echo "============================================="
echo "  Setup complete!"
echo "============================================="
echo ""
echo "Add these GitHub repo secrets:"
echo "  https://github.com/$GITHUB_REPO/settings/secrets/actions"
echo ""
echo "  VPS_HOST     = $(hostname -I | awk '{print $1}')"
echo "  VPS_USER     = $DEPLOY_USER"
if [[ -f "$CI_KEY" ]]; then
  echo "  VPS_SSH_KEY  = (contents of $CI_KEY)"
  echo ""
  echo "To show the private key:"
  echo "  cat $CI_KEY"
else
  echo "  VPS_SSH_KEY  = (your CI deploy private key)"
fi
echo ""
echo "Edit secrets in: $ENV_FILE"
echo ""
echo "First deploy (manual):"
echo "  sudo -u $DEPLOY_USER bash -c 'cd $PROJECT_DIR && docker compose -f docker-compose.prod.yml pull && docker compose -f docker-compose.prod.yml up -d'"
echo ""
echo "After that, every push to master triggers automatic deployment."
