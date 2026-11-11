#!/bin/bash
# Generated bot games: the C# ServerOracle lets the server's bots play whole games through the hub
# (random settings, every seat a bot, the virtual clock moved to each delayed call in turn), runs
# them through the Rust hub, compares, and deletes everything it wrote. Runs through
# rust/tools/heavy.sh.
#
#   rust/server-equiv/bot-check.sh [--seed S] [--games N] [--start I] [--max-steps N]
set -e -o pipefail
here="$(cd "$(dirname "$0")" && pwd)"
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
export TZ=Etc/GMT-2
cd "$here/.."
dotnet build -c Release -v q --nologo ../treachery-online/ServerOracle/Treachery.ServerOracle.csproj >/dev/null
cargo build -q --release -p server-equiv
out=$(mktemp -d -t server-equiv-bots.XXXXXX)
trap 'rm -rf "$out"' EXIT
tools/heavy.sh dotnet ../treachery-online/ServerOracle/bin/Release/net10.0/Treachery.ServerOracle.dll botgames --out "$out" "$@"
tools/heavy.sh ./target/release/server-equiv run --scenarios "$out" | grep -v -E "^(Restore|Persist|Cleanup|Invalid bot decision|Timer:|Unable to restore)"
