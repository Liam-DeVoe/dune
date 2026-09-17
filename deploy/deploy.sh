#!/usr/bin/env bash
# Push the local source tree to the VM, build it there, and restart the service.
# Safe to run repeatedly. The database in /opt/treachery/data is never touched.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

echo "==> syncing $SRC_DIR to $VM_HOST:$REMOTE_SRC"
rsync -az --delete \
    --exclude '.git' --exclude '.idea' --exclude '.DS_Store' \
    --exclude 'bin/' --exclude 'obj/' --exclude '*.db*' \
    -e "ssh -o BatchMode=yes -i $VM_SSH_KEY" \
    "$SRC_DIR/" "$VM_USER@$VM_HOST:$REMOTE_SRC/"

echo "==> publishing on the VM (takes about a minute)"
vm_ssh bash -s <<EOF
set -euo pipefail
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
cd $REMOTE_SRC
rm -rf $REMOTE_APP.new
dotnet publish Server/Treachery.Server.csproj -c Release -o $REMOTE_APP.new 2>&1 | grep -Ev 'warning (CS|BL)' | tail -15

# Swap the build in under the service so a failed publish never leaves a broken app dir behind.
sudo systemctl stop treachery.service
rm -rf $REMOTE_APP.old
[ -d $REMOTE_APP ] && mv $REMOTE_APP $REMOTE_APP.old
mv $REMOTE_APP.new $REMOTE_APP
sudo systemctl start treachery.service
EOF

echo "==> waiting for the service"
for i in $(seq 1 30); do
    if vm_ssh "curl -sf -o /dev/null http://127.0.0.1:$APP_PORT/"; then
        echo "==> up: https://$VM_HOST/"
        exit 0
    fi
    sleep 2
done

echo "service did not come up; recent log:" >&2
vm_ssh "sudo journalctl -u treachery --no-pager -n 40" >&2
exit 1
