#!/usr/bin/env bash
# Create the VM on exe.dev from nothing and bring the site up on it.
# Idempotent: rerunning on an existing VM just reinstalls and redeploys.
#
# What it does:
#   1. creates the VM (2 cpu, 8 GB, 30 GB disk) if it doesn't exist
#   2. points https://<vm>.exe.xyz/ at the app port and makes it public
#   3. installs the .NET 10 SDK, sqlite3 and rsync on the VM
#   4. installs the systemd unit
#   5. runs deploy.sh to build and start the app
#   6. creates the admin account with a random password (saved to deploy/.admin-password)
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

echo "==> exposing port $APP_PORT publicly"
exe share port "$VM_NAME" "$APP_PORT"
exe share set-public "$VM_NAME"

echo "==> installing .NET SDK, sqlite3, rsync, and the systemd unit"
vm_ssh bash -s <<EOF
set -euo pipefail
sudo mkdir -p $REMOTE_SRC $REMOTE_DATA
sudo chown -R $VM_USER:$VM_USER /opt/treachery
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq sqlite3 rsync >/dev/null
if ! command -v dotnet >/dev/null || ! dotnet --list-sdks | grep -q '^10\.'; then
    curl -fsSL https://dot.net/v1/dotnet-install.sh -o /tmp/dotnet-install.sh
    sudo bash /tmp/dotnet-install.sh --channel 10.0 --install-dir /usr/share/dotnet >/dev/null
    sudo ln -sf /usr/share/dotnet/dotnet /usr/bin/dotnet
fi
dotnet --version
EOF
vm_ssh "sudo tee /etc/systemd/system/treachery.service >/dev/null && sudo systemctl daemon-reload && sudo systemctl enable treachery.service" \
    < "$REPO_ROOT/deploy/treachery.service"

"$REPO_ROOT/deploy/deploy.sh"

# The app makes whoever registers the username in GameAdminUsername (default "admin")
# the site admin, so claim it before anyone else can. Passwords are sha256 hex of the
# plaintext, hashed in the browser (Client/Support.cs GetHash).
PW_FILE="$REPO_ROOT/deploy/.admin-password"
if vm_ssh "sqlite3 $REMOTE_DATA/treachery.db \"SELECT 1 FROM Users WHERE Name='admin'\"" | grep -q 1; then
    echo "==> admin account already exists"
else
    PW="$(openssl rand -base64 18 | tr -d '/+=' | cut -c1-20)"
    HASH="$(printf '%s' "$PW" | shasum -a 256 | cut -d' ' -f1)"
    vm_ssh "sqlite3 $REMOTE_DATA/treachery.db \"INSERT INTO Users (Name, Email, HashedPassword, PasswordResetToken, PasswordResetTokenCreated, PlayerName, LastLogin) VALUES ('admin', '', '$HASH', '', '0001-01-01 00:00:00+00:00', 'Admin', datetime('now'));\""
    (umask 077; printf '%s\n' "$PW" > "$PW_FILE")
    echo "==> admin account created; password saved to $PW_FILE"
fi

echo "==> done: https://$VM_HOST/"
