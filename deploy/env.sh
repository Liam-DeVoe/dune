# shellcheck shell=bash
# Shared settings for the deploy scripts. Sourced, not executed.
# Override any of these in your shell before running a script.

# Name of the VM on exe.dev. Its public URL is https://$VM_NAME.exe.xyz/.
VM_NAME="${VM_NAME:-treachery}"

# ssh alias for the exe.dev control plane (the account the VM lives under).
EXE_ACCOUNT="${EXE_ACCOUNT:-exe-personal}"

# Key for the VM itself. ~/.ssh/config maps *.exe.xyz to a different account's
# key, so we pass this one explicitly.
VM_SSH_KEY="${VM_SSH_KEY:-$HOME/.ssh/id_ed25519_exepersonal}"

VM_HOST="$VM_NAME.exe.xyz"
VM_USER="exedev"

# exe.dev proxies https://$VM_HOST/ to Caddy on PROXY_PORT (deploy/Caddyfile), which sends /beta to
# the Rust server on BETA_PORT and everything else to the C# server on APP_PORT.
PROXY_PORT=8080
APP_PORT=8000
BETA_PORT=8001

# Layout on the VM.
REMOTE_SRC=/opt/treachery/src    # rsync target for the source tree
REMOTE_APP=/opt/treachery/app    # dotnet publish output, what the service runs
REMOTE_DATA=/opt/treachery/data  # sqlite database, survives redeploys
REMOTE_RUST_SRC=/opt/treachery/rust-src  # rsync target for rust/, built there
REMOTE_BETA=/opt/treachery/beta          # the Rust server binary and web/ build (web/)
REMOTE_DATA_BETA=/opt/treachery/data-beta  # the beta's own sqlite database

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$REPO_ROOT/treachery-online"

vm_ssh() {
    ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -i "$VM_SSH_KEY" "$VM_USER@$VM_HOST" "$@"
}

exe() {
    ssh -o BatchMode=yes "$EXE_ACCOUNT" "$@"
}
