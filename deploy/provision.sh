#!/usr/bin/env bash
# Create the VM on exe.dev from nothing and bring the site up on it.
# Idempotent: rerunning on an existing VM just reinstalls and redeploys.
#
# What it does:
#   1. creates the VM (2 cpu, 8 GB, 30 GB disk) if it doesn't exist, and makes it public
#   2. installs the .NET 10 SDK, Rust, Caddy, sqlite3 and rsync on the VM
#   3. installs the systemd units and the Caddyfile
#   4. runs deploy.sh (the C# app) and deploy-beta.sh (the Rust server and web client, at /beta/)
#   5. creates an admin account in each database, with a random password
#      (saved to deploy/.admin-password and deploy/.admin-password-beta)
#   6. points https://<vm>.exe.xyz/ at Caddy, once both apps answer through it
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

if exe ls --json | grep -q "\"vm_name\":\"$VM_NAME\""; then
    echo "==> VM $VM_NAME already exists"
else
    echo "==> creating VM $VM_NAME"
    exe new --name="$VM_NAME" --cpu=2 --memory=8GB --disk=30GB --no-email --comment=treachery-online-mirror
    # DNS and sshd need a moment.
    for _ in $(seq 1 30); do vm_ssh true 2>/dev/null && break; sleep 5; done
fi

exe share set-public "$VM_NAME"

echo "==> installing .NET SDK, Rust, Caddy, sqlite3, rsync, and the systemd units"
vm_ssh bash -s <<EOF
set -euo pipefail
sudo mkdir -p $REMOTE_SRC $REMOTE_DATA $REMOTE_RUST_SRC $REMOTE_BETA $REMOTE_DATA_BETA
sudo chown -R $VM_USER:$VM_USER /opt/treachery
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq sqlite3 rsync caddy build-essential pkg-config >/dev/null
if [ ! -x ~/.cargo/bin/cargo ]; then
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
fi
~/.cargo/bin/cargo --version
if ! command -v dotnet >/dev/null || ! dotnet --list-sdks | grep -q '^10\.'; then
    curl -fsSL https://dot.net/v1/dotnet-install.sh -o /tmp/dotnet-install.sh
    sudo bash /tmp/dotnet-install.sh --channel 10.0 --install-dir /usr/share/dotnet >/dev/null
    sudo ln -sf /usr/share/dotnet/dotnet /usr/bin/dotnet
fi
dotnet --version
EOF
for unit in treachery treachery-beta; do
    vm_ssh "sudo tee /etc/systemd/system/$unit.service >/dev/null && sudo systemctl daemon-reload && sudo systemctl enable $unit.service" \
        < "$REPO_ROOT/deploy/$unit.service"
done
vm_ssh "sudo tee /etc/caddy/Caddyfile >/dev/null && caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null && sudo systemctl enable caddy && sudo systemctl reload-or-restart caddy" \
    < "$REPO_ROOT/deploy/Caddyfile"

"$REPO_ROOT/deploy/deploy.sh"
"$REPO_ROOT/deploy/deploy-beta.sh"

# The app makes whoever registers the username in GameAdminUsername (default "admin")
# the site admin, so claim it before anyone else can. Passwords are sha256 hex of the
# plaintext, hashed in the browser (Client/Support.cs GetHash, web/src/state/connection.ts).
# Dates carry an offset, as EF Core writes a DateTimeOffset. Both servers read one without as UTC,
# but this keeps the row in the format EF itself writes.
create_admin() {
    local db="$1" pw_file="$2"
    if vm_ssh "sqlite3 $db \"SELECT 1 FROM Users WHERE Name='admin'\"" | grep -q 1; then
        echo "==> admin account in $db already exists"
        return
    fi
    local pw hash
    pw="$(openssl rand -base64 18 | tr -d '/+=' | cut -c1-20)"
    hash="$(printf '%s' "$pw" | shasum -a 256 | cut -d' ' -f1)"
    vm_ssh "sqlite3 $db \"INSERT INTO Users (Name, Email, HashedPassword, PasswordResetToken, PasswordResetTokenCreated, PlayerName, LastLogin) VALUES ('admin', '', '$hash', '', '0001-01-01 00:00:00+00:00', 'Admin', datetime('now') || '+00:00');\""
    (umask 077; printf '%s\n' "$pw" > "$pw_file")
    echo "==> admin account created in $db; password saved to $pw_file"
}
create_admin "$REMOTE_DATA/treachery.db" "$REPO_ROOT/deploy/.admin-password"
create_admin "$REMOTE_DATA_BETA/treachery.db" "$REPO_ROOT/deploy/.admin-password-beta"

# Switch the public URL to Caddy only once both apps answer through it.
echo "==> checking both apps through Caddy"
for _ in $(seq 1 30); do
    if vm_ssh "curl -sf -o /dev/null http://127.0.0.1:$PROXY_PORT/ && curl -sf -o /dev/null http://127.0.0.1:$PROXY_PORT/beta/"; then
        break
    fi
    sleep 2
done
vm_ssh "curl -sf -o /dev/null http://127.0.0.1:$PROXY_PORT/ && curl -sf -o /dev/null http://127.0.0.1:$PROXY_PORT/beta/"
echo "==> pointing https://$VM_HOST/ at port $PROXY_PORT"
exe share port "$VM_NAME" "$PROXY_PORT"

echo "==> done: https://$VM_HOST/ and https://$VM_HOST/beta/"
