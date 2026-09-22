#!/usr/bin/env bash
# Build the Blazor client. Serialised with a lock so several people (or agents)
# can run it at once without MSBuild trampling obj/ and bin/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export DOTNET_ROOT="${DOTNET_ROOT:-$HOME/dotnet10}" DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
export PATH="$DOTNET_ROOT:$PATH"
LOCK=/tmp/treachery-build.lock
for _ in $(seq 1 600); do mkdir "$LOCK" 2>/dev/null && break; sleep 1; done
trap 'rmdir "$LOCK" 2>/dev/null' EXIT
cd "$ROOT/treachery-online"
dotnet build Client/Treachery.Client.csproj 2>&1 | grep -E ' error |Build succeeded|Error\(s\)|Warning\(s\)' | sort -u
