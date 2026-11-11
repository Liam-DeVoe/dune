#!/bin/bash
# Regenerates rust/docs/client-inventory.md and rust/view/csharp-inventory/out/client-inventory.json.
# Builds the Blazor client once with the Razor generator's output written to a temp dir (about a minute).
set -euo pipefail
export DOTNET_ROOT="$HOME/dotnet10" PATH="$HOME/dotnet10:$PATH"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
client="$root/treachery-online/Client"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# -t:Rebuild so the generator runs even when the client is up to date.
dotnet build "$client/Treachery.Client.csproj" -nologo -v:q -t:Rebuild \
  -p:EmitCompilerGeneratedFiles=true -p:CompilerGeneratedFilesOutputPath="$tmp/gen" \
  -getItem:ReferencePathWithRefAssemblies > "$tmp/build.out" 2>&1
# The item list is the JSON object at the end of the output.
python3 - "$tmp/build.out" "$tmp/refs.txt" <<'PY'
import json, sys
text = open(sys.argv[1]).read()
start = text.index('\n{') + 1 if not text.startswith('{') else 0
items = json.loads(text[start:])["Items"]["ReferencePathWithRefAssemblies"]
open(sys.argv[2], "w").write("\n".join(i["FullPath"] for i in items))
PY
mkdir -p "$here/out"
dotnet run --project "$here" -c Release -- "$client" "$tmp/gen" "$tmp/refs.txt" \
  "$here/out/client-inventory.json" "$root/rust/docs/client-inventory.md" "$here/areas.tsv"
