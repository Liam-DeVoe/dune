#!/usr/bin/env bash
# Measures C# line coverage of Treachery.Shared over a bots + fuzz corpus run.
# Usage: Oracle/coverage.sh [BOT_GAMES] [FUZZ_GAMES] [SEED] [WORKDIR]
# Needs coverlet.console: dotnet tool install --global coverlet.console
set -euo pipefail

BOT_GAMES=${1:-300}
FUZZ_GAMES=${2:-200}
SEED=${3:-100}
WORK=${4:-$(mktemp -d)}
# Fuzzing finds real endless loops; under coverlet a game is given up after this many seconds.
HANG_SECONDS=${HANG_SECONDS:-30}

export DOTNET_ROOT="${DOTNET_ROOT:-$HOME/dotnet10}"
export PATH="$DOTNET_ROOT:$PATH:$HOME/.dotnet/tools"
HERE="$(cd "$(dirname "$0")" && pwd)"

dotnet build -c Release "$HERE/Treachery.Oracle.csproj" -nologo -v quiet

# coverlet instruments the assemblies in place, so work on a copy of the build output.
mkdir -p "$WORK"
rm -rf "$WORK/bin"
cp -R "$HERE/bin/Release/net10.0" "$WORK/bin"
ORACLE="$WORK/bin/Treachery.Oracle.dll"

coverlet "$WORK/bin" --target dotnet \
  --targetargs "$ORACLE bots --seed $SEED --games $BOT_GAMES --out $WORK/bots --hash none --discard true" \
  --include "[Treachery.Shared]*" --single-hit -f json -o "$WORK/bots.json"

coverlet "$WORK/bin" --target dotnet \
  --targetargs "$ORACLE fuzz --seed $SEED --games $FUZZ_GAMES --out $WORK/fuzz --hash none --discard true --hang-seconds $HANG_SECONDS" \
  --include "[Treachery.Shared]*" --single-hit --merge-with "$WORK/bots.json" \
  -f cobertura -o "$WORK/coverage.cobertura.xml" || echo "fuzz run exited with $? (4 means some games hung; their coverage up to the hang is included)"

dotnet "$HERE/bin/Release/net10.0/Treachery.Oracle.dll" coverage --in "$WORK/coverage.cobertura.xml"
rm -rf "$WORK/bin" "$WORK/bots.json" "$WORK/bots" "$WORK/fuzz"
echo "Cobertura report: $WORK/coverage.cobertura.xml"
