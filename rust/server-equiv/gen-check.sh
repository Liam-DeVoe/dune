#!/bin/bash
# Generated game scenarios: turns oracle traces into hub scenarios with the C# ServerOracle
# (N seated users play the trace's events through SubmitEvent), runs them through the Rust hub,
# compares, and deletes everything it wrote. Runs through rust/tools/heavy.sh.
#
#   rust/server-equiv/gen-check.sh rust/traces/sample [generator options]
#
# Generator options: --max-steps N, --advance-seconds S (default 20), --async-play true,
# --restart-every K. See `dotnet Treachery.ServerOracle.dll --help`.
set -e
here="$(cd "$(dirname "$0")" && pwd)"
traces="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
shift
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
export TZ=Etc/GMT-2
cd "$here/.."
dotnet build -c Release -v q --nologo ../treachery-online/ServerOracle/Treachery.ServerOracle.csproj >/dev/null
cargo build -q --release -p server-equiv
out=$(mktemp -d -t server-equiv-gen.XXXXXX)
trap 'rm -rf "$out"' EXIT
tools/heavy.sh dotnet ../treachery-online/ServerOracle/bin/Release/net10.0/Treachery.ServerOracle.dll generate --in "$traces" --out "$out" "$@"
tools/heavy.sh ./target/release/server-equiv run --scenarios "$out"
