#!/bin/bash
# Builds treachery-online/ServerOracle and rewrites the golden C# transcripts of the hand-written
# scenarios (scenarios/*.jsonl -> golden/*.transcript.jsonl). Run it after changing a scenario or
# the C# server. Extra arguments go to `ServerOracle run` (for example `--full true`).
#
#   rust/server-equiv/run-oracle.sh                 # every scenario
#   rust/server-equiv/run-oracle.sh --in scenarios/lobby.jsonl
set -e
cd "$(dirname "$0")"
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
# Scenarios start at +02:00; the C# server converts DateTime.Now with the machine's zone.
export TZ=Etc/GMT-2
dotnet build -c Release -v q --nologo ../../treachery-online/ServerOracle/Treachery.ServerOracle.csproj >/dev/null
oracle=../../treachery-online/ServerOracle/bin/Release/net10.0/Treachery.ServerOracle.dll
if [[ " $* " == *" --in "* ]]; then
  dotnet "$oracle" run --out golden "$@"
else
  dotnet "$oracle" run --in scenarios --out golden "$@"
fi
