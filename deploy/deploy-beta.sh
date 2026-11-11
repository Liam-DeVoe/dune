#!/usr/bin/env bash
# Deploy the beta at https://$VM_HOST/beta/: the Rust server (rust/server), built on the VM, and the
# React client (web/), built here with base /beta/. Restarts treachery-beta.service.
# The beta database in /opt/treachery/data-beta is never touched.
#
# Needs the C# app deployed first: the beta serves its art and CSS from /opt/treachery/app/wwwroot.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

WEB_OUT="$(mktemp -d)"
trap 'rm -rf "$WEB_OUT"' EXIT

echo "==> building web/ with base /beta/"
(
    cd "$REPO_ROOT/web"
    [ -d node_modules ] || npm ci
    npx tsc --noEmit
    TREACHERY_BASE=/beta/ npx vite build --outDir "$WEB_OUT" --emptyOutDir --logLevel warn
)

echo "==> syncing rust/ and the web build to $VM_HOST"
RSYNC_SSH="ssh -o BatchMode=yes -i $VM_SSH_KEY"
vm_ssh "mkdir -p $REMOTE_RUST_SRC $REMOTE_BETA $REMOTE_DATA_BETA"
rsync -az --delete --exclude 'target/' --exclude 'traces/' --exclude '.DS_Store' \
    -e "$RSYNC_SSH" "$REPO_ROOT/rust/" "$VM_USER@$VM_HOST:$REMOTE_RUST_SRC/"
rsync -az --delete -e "$RSYNC_SSH" "$WEB_OUT/" "$VM_USER@$VM_HOST:$REMOTE_BETA/web.new/"

echo "==> building the server on the VM (a few minutes from scratch, less after)"
vm_ssh bash -s <<EOF
set -euo pipefail
source ~/.cargo/env
cd $REMOTE_RUST_SRC
cargo build --release -p treachery-server 2>&1 | tail -3

# Swap both in under the service, as deploy.sh does.
sudo systemctl stop treachery-beta.service
cp target/release/treachery-server $REMOTE_BETA/treachery-server
rm -rf $REMOTE_BETA/web.old
[ -d $REMOTE_BETA/web ] && mv $REMOTE_BETA/web $REMOTE_BETA/web.old
mv $REMOTE_BETA/web.new $REMOTE_BETA/web
sudo systemctl start treachery-beta.service
EOF

echo "==> waiting for the service"
for _ in $(seq 1 30); do
    if vm_ssh "curl -sf -o /dev/null http://127.0.0.1:$BETA_PORT/"; then
        echo "==> up: https://$VM_HOST/beta/"
        exit 0
    fi
    sleep 2
done

echo "service did not come up; recent log:" >&2
vm_ssh "sudo journalctl -u treachery-beta --no-pager -n 40" >&2
exit 1
