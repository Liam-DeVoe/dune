#!/usr/bin/env bash
# Publish the Blazor client on its own (no server) to /tmp/treachery-client-<label>,
# for scripts/ui/shots.py to serve. The label defaults to "after"; run this from
# a git worktree of an older commit with the label "before" to get the other half
# of a before/after pair. Takes the same lock as build-client.sh, so it waits for
# any other build of this tree instead of trampling obj/ and bin/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export DOTNET_ROOT="${DOTNET_ROOT:-$HOME/dotnet10}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
export PATH="$DOTNET_ROOT:$PATH"
LABEL="${1:-after}"
OUT="/tmp/treachery-client-$LABEL"
LOCK=/tmp/treachery-build.lock
for _ in $(seq 1 600); do mkdir "$LOCK" 2>/dev/null && break; sleep 1; done
trap 'rmdir "$LOCK" 2>/dev/null' EXIT
cd "$ROOT/treachery-online"
# Publish into a fresh directory so a file removed from the client cannot linger from an older publish.
STAGING="$OUT.publishing"
rm -rf "$STAGING"
LOG="$STAGING.log"
if ! dotnet publish Client/Treachery.Client.csproj -c Debug -o "$STAGING" > "$LOG" 2>&1; then
  grep -E ' error |Error\(s\)' "$LOG" | sort -u >&2
  echo "publish failed; full log in $LOG" >&2; rm -rf "$STAGING"; exit 1
fi
grep -E 'Warning\(s\)|Error\(s\)' "$LOG" | sort -u || true
[ -f "$STAGING/wwwroot/index.html" ] || { echo "publish failed: no wwwroot/index.html in $STAGING" >&2; exit 1; }
rm -rf "$OUT"; mv "$STAGING" "$OUT"
echo "published $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo '?') to $OUT"
