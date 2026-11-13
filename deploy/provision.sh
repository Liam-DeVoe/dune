#!/usr/bin/env bash
# Create the VM on exe.dev from nothing and bring the site up on it.
# Idempotent: rerunning on an existing VM just reinstalls and redeploys.
#
# What it does:
#   1. creates the VM (2 cpu, 8 GB, 30 GB disk) if it doesn't exist, and makes it public
#   2. installs Rust, Caddy, sqlite3 and rsync on the VM
#   3. runs deploy.sh, which builds and starts the site and installs the systemd unit and Caddyfile
#   4. creates an admin account in the database with a random password, saved to
#      deploy/.admin-password, unless the database already has one
#   5. points https://<vm>.exe.xyz/ at Caddy
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

if exe ls --json | grep -q "\"vm_name\":\"$VM_NAME\""; then
    echo "==> VM $VM_NAME already exists"
else
    echo "==> creating VM $VM_NAME"
    exe new --name="$VM_NAME" --cpu=2 --memory=8GB --disk=30GB --no-email --comment=treachery-online
    # DNS and sshd need a moment.
    for _ in $(seq 1 30); do vm_ssh true 2>/dev/null && break; sleep 5; done
fi

exe share set-public "$VM_NAME"

echo "==> installing Rust, Caddy, sqlite3 and rsync"
vm_ssh bash -s <<EOF
set -euo pipefail
sudo mkdir -p /opt/treachery
sudo chown $VM_USER:$VM_USER /opt/treachery
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq sqlite3 rsync caddy build-essential pkg-config >/dev/null
if [ ! -x ~/.cargo/bin/cargo ]; then
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
fi
~/.cargo/bin/cargo --version
sudo systemctl enable caddy
EOF

"$REPO_ROOT/deploy/deploy.sh"

# The server makes whoever registers the username in TREACHERY_ADMIN ("admin" in
# deploy/treachery.service) the site admin, so claim it before anyone else can. Passwords are
# sha256 hex of the plaintext, hashed in the browser (web/src/state/connection.ts). Times are
# RFC 3339 in UTC (rust/server/src/db.rs). The server created the database on its first start.
DB="$REMOTE_DATA/treachery.db"
PW_FILE="$REPO_ROOT/deploy/.admin-password"
if vm_ssh "sqlite3 $DB \"SELECT 1 FROM users WHERE name='admin'\"" | grep -q 1; then
    echo "==> admin account already exists"
else
    pw="$(openssl rand -base64 18 | tr -d '/+=' | cut -c1-20)"
    hash="$(printf '%s' "$pw" | shasum -a 256 | cut -d' ' -f1)"
    vm_ssh "sqlite3 $DB \"INSERT INTO users (name, password_hash, email, player_name, last_login) VALUES ('admin', '$hash', '', 'Admin', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));\""
    (umask 077; printf '%s\n' "$pw" > "$PW_FILE")
    echo "==> admin account created; password saved to $PW_FILE"
fi

echo "==> pointing https://$VM_HOST/ at port $PROXY_PORT"
exe share port "$VM_NAME" "$PROXY_PORT"

echo "==> done: https://$VM_HOST/"
