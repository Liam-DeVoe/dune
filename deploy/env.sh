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

# Port the app listens on inside the VM. exe.dev proxies https://$VM_HOST/ to it.
APP_PORT=8000

# Layout on the VM.
REMOTE_SRC=/opt/treachery/src    # rsync target for the source tree
REMOTE_APP=/opt/treachery/app    # dotnet publish output, what the service runs
REMOTE_DATA=/opt/treachery/data  # sqlite database, survives redeploys

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$REPO_ROOT/treachery-online"

vm_ssh() {
    ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -i "$VM_SSH_KEY" "$VM_USER@$VM_HOST" "$@"
}

exe() {
    ssh -o BatchMode=yes "$EXE_ACCOUNT" "$@"
}
