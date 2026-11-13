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

# exe.dev proxies https://$VM_HOST/ to Caddy on PROXY_PORT (deploy/Caddyfile), which sends
# everything to the Rust server on APP_PORT (deploy/treachery.service).
PROXY_PORT=8080
APP_PORT=8000

# Layout on the VM.
REMOTE_SRC=/opt/treachery/rust-src   # rsync target for rust/, built there with cargo
REMOTE_SERVER=/opt/treachery/server  # what the service runs: treachery-server, web/, assets/
REMOTE_DATA=/opt/treachery/db        # sqlite database, survives redeploys

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The art, fonts, scripts and SCSS the web client links. They still live in the old C# client's
# tree; deploy.sh copies them from here and compiles the SCSS.
ASSETS_SRC="$REPO_ROOT/treachery-online/Client/wwwroot"
STYLES_SRC="$REPO_ROOT/treachery-online/Client/Styles"
# The Dart Sass version the C# build used (AspNetCore.SassCompiler 1.103.0).
SASS_VERSION=1.103.0

vm_ssh() {
    ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -i "$VM_SSH_KEY" "$VM_USER@$VM_HOST" "$@"
}

exe() {
    ssh -o BatchMode=yes "$EXE_ACCOUNT" "$@"
}
