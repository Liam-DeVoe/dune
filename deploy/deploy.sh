#!/usr/bin/env bash
# Deploy the site to https://$VM_HOST/: the Rust server (rust/server, built on the VM), the React
# client (web/, built here) and the art, CSS and fonts it links. Also installs
# deploy/treachery.service and deploy/Caddyfile when they changed. Safe to run repeatedly.
# The database in $REMOTE_DATA is never touched.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

echo "==> building web/"
(
    cd "$REPO_ROOT/web"
    # Reinstall when package-lock.json is newer than the last install.
    [ node_modules/.package-lock.json -nt package-lock.json ] || npm ci
    npx tsc --noEmit
    npx vite build --outDir "$STAGE/web" --emptyOutDir --logLevel warn
)

echo "==> collecting assets and compiling the stylesheet"
mkdir -p "$STAGE/assets/css"
rsync -a --exclude '.DS_Store' --exclude '/css/treachery.css' "$ASSETS_SRC/" "$STAGE/assets/"
# Dart Sass, at the version pinned in web/package.json.
(cd "$REPO_ROOT/web" && npx --no-install sass --style=expanded --no-source-map \
    "$STYLES_SRC/treachery.scss" "$STAGE/assets/css/treachery.css")

echo "==> syncing rust/, the web build and the assets to $VM_HOST"
RSYNC_SSH="ssh -o BatchMode=yes -i $VM_SSH_KEY"
vm_ssh "sudo mkdir -p $REMOTE_SRC $REMOTE_SERVER/incoming $REMOTE_DATA && sudo chown $VM_USER:$VM_USER $REMOTE_SRC $REMOTE_SERVER $REMOTE_SERVER/incoming $REMOTE_DATA"
rsync -az --delete --exclude 'target/' --exclude 'traces/' --exclude '.DS_Store' \
    -e "$RSYNC_SSH" "$REPO_ROOT/rust/" "$VM_USER@$VM_HOST:$REMOTE_SRC/"
# Into incoming/, which persists, so later deploys send only what changed.
rsync -az --delete -e "$RSYNC_SSH" "$STAGE/web/" "$VM_USER@$VM_HOST:$REMOTE_SERVER/incoming/web/"
rsync -az --delete -e "$RSYNC_SSH" "$STAGE/assets/" "$VM_USER@$VM_HOST:$REMOTE_SERVER/incoming/assets/"
rsync -az -e "$RSYNC_SSH" "$REPO_ROOT/deploy/treachery.service" "$REPO_ROOT/deploy/Caddyfile" \
    "$VM_USER@$VM_HOST:$REMOTE_SERVER/incoming/"

echo "==> building the server on the VM (a few minutes from scratch, less after)"
vm_ssh bash -s <<EOF
set -euo pipefail
source ~/.cargo/env
cd $REMOTE_SRC
cargo build --release -p treachery-server 2>&1 | tail -3

if ! cmp -s $REMOTE_SERVER/incoming/treachery.service /etc/systemd/system/treachery.service; then
    echo "==> installing treachery.service"
    sudo install -m 644 $REMOTE_SERVER/incoming/treachery.service /etc/systemd/system/treachery.service
    sudo systemctl daemon-reload
    sudo systemctl enable treachery.service
fi

# Swap everything in while the service is stopped, so it never runs a mix of old and new.
sudo systemctl stop treachery.service
cp target/release/treachery-server $REMOTE_SERVER/treachery-server
rsync -a --delete $REMOTE_SERVER/incoming/web/ $REMOTE_SERVER/web/
rsync -a --delete $REMOTE_SERVER/incoming/assets/ $REMOTE_SERVER/assets/
sudo systemctl start treachery.service
EOF

echo "==> waiting for the service"
up=
for _ in $(seq 1 30); do
    if vm_ssh "curl -sf -o /dev/null http://127.0.0.1:$APP_PORT/"; then up=1; break; fi
    sleep 2
done
if [ -z "$up" ]; then
    echo "service did not come up; recent log:" >&2
    vm_ssh "sudo journalctl -u treachery --no-pager -n 40" >&2
    exit 1
fi

vm_ssh bash -s <<EOF
set -euo pipefail
if ! cmp -s $REMOTE_SERVER/incoming/Caddyfile /etc/caddy/Caddyfile; then
    echo "==> installing the Caddyfile"
    if ! out="\$(caddy validate --config $REMOTE_SERVER/incoming/Caddyfile --adapter caddyfile 2>&1)"; then
        echo "\$out" >&2
        exit 1
    fi
    sudo install -m 644 $REMOTE_SERVER/incoming/Caddyfile /etc/caddy/Caddyfile
    sudo systemctl reload-or-restart caddy
fi
curl -sf -o /dev/null http://127.0.0.1:$PROXY_PORT/
EOF
echo "==> up: https://$VM_HOST/"
