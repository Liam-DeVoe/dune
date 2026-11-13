#!/bin/bash
# Builds the reference binary of the observable-behavior harness (rust/docs/native-harness.md):
# the `observe` recorder built from a frozen copy of the engine, bots and view crates. Every
# `observe live` and `observe first` run compares the working tree against it.
#
#   rust/tools/build-native-ref.sh                 # from the working tree as it is now
#   rust/tools/build-native-ref.sh --rev TAG       # from a git revision (must contain rust/observe)
#   rust/tools/build-native-ref.sh --archive FILE  # from a source archive this script saved
#   rust/tools/build-native-ref.sh --source DIR    # from a frozen copy of rust/ (DIR/engine, ...)
#
# The current reference is built from ~/treachery-native-ref/rust, a copy of rust/ frozen on
# 2026-11-11 before step 2 started, with rust/observe copied in (native-harness.md).
#
# Rebuild it only on purpose: when a behavior change has been reviewed and accepted
# (native-harness.md, "Deliberate behavior changes"). Rebuilding from a working tree that has an
# unreviewed change makes that change invisible to the harness.
#
# Output, in rust/reference/ (not in git), with a copy of the binary and SOURCE in
# ~/treachery-native-ref/ for git worktrees:
#   observe          the reference binary
#   source.tar.gz    the sources it was built from (rebuild with --archive)
#   SOURCE           where the sources came from, when, and the binary's sha256
#
# The build uses one temporary target dir under $TMPDIR (about 1.5 GB), deleted at the end.

set -euo pipefail

rust_dir="$(cd "$(dirname "$0")/.." && pwd)"
repo="$(cd "$rust_dir/.." && pwd)"
out="$rust_dir/reference"
# The crates the observe binary needs. Older sources, such as the frozen copy the current reference
# is built from, also have canon-derive and differ; each mode keeps the ones its source has.
all_crates="bots canon-derive differ dotnet-compat engine observe skin view"
crates_in() { for c in $all_crates; do if [ -f "$1/$c/Cargo.toml" ]; then echo "$c"; fi; done; }

mode=tree
arg=""
case "${1:-}" in
  "") ;;
  --rev) mode=rev; arg="${2:?--rev needs a revision}" ;;
  --source) mode=source; arg="$(cd "${2:?--source needs a directory}" && pwd)" ;;
  --archive) mode=archive; arg="$(cd "$(dirname "${2:?--archive needs a file}")" && pwd)/$(basename "$2")" ;;
  *) echo "usage: $0 [--rev REV | --archive FILE | --source DIR]" >&2; exit 2 ;;
esac

avail_kb=$(df -k "${TMPDIR:-/tmp}" | awk 'NR==2 {print $4}')
if [ "$avail_kb" -lt 3000000 ]; then
  echo "build-native-ref.sh: less than 3 GB free on the temp disk; not building (DEVELOPMENT.md)" >&2
  exit 1
fi

work=$(mktemp -d -t native-ref.XXXXXX)
trap 'rm -rf "$work"' EXIT
src="$work/rust"
mkdir -p "$src"

case $mode in
  tree)
    crates=$(crates_in "$rust_dir")
    for c in $crates; do
      mkdir -p "$src/$c"
      cp "$rust_dir/$c/Cargo.toml" "$src/$c/"
      cp -R "$rust_dir/$c/src" "$src/$c/"
    done
    cp "$rust_dir/Cargo.toml" "$rust_dir/Cargo.lock" "$src/"
    head=$(git -C "$repo" rev-parse HEAD)
    dirty=$(git -C "$repo" status --porcelain -- $(for c in $crates; do echo "rust/$c"; done) | wc -l | tr -d ' ')
    origin="working tree at $head ($dirty changed or new files in the crates)"
    ;;
  source)
    crates=$(crates_in "$arg")
    for c in $crates; do
      mkdir -p "$src/$c"
      cp "$arg/$c/Cargo.toml" "$src/$c/"
      cp -R "$arg/$c/src" "$src/$c/"
    done
    cp "$arg/Cargo.toml" "$arg/Cargo.lock" "$src/"
    origin="frozen copy $arg"
    ;;
  rev)
    crates=$(for c in $all_crates; do if git -C "$repo" cat-file -e "$arg:rust/$c/Cargo.toml" 2>/dev/null; then echo "$c"; fi; done)
    git -C "$repo" archive "$arg" rust/Cargo.toml rust/Cargo.lock $(for c in $crates; do echo "rust/$c/Cargo.toml rust/$c/src"; done) | tar -x -C "$work"
    origin="git revision $arg ($(git -C "$repo" rev-parse "$arg"))"
    ;;
  archive)
    tar -xzf "$arg" -C "$work"
    crates=$(crates_in "$src")
    origin="archive $arg ($(cat "$src/ORIGIN" 2>/dev/null || echo unknown origin))"
    ;;
esac

# A workspace of just these crates (the server and other crates are not needed).
members=$(for c in $crates; do printf '"%s", ' "$c"; done)
sed -i '' "s|^members = .*|members = [${members%, }]|" "$src/Cargo.toml"
echo "$origin" > "$src/ORIGIN"

echo "build-native-ref.sh: building from $origin" >&2
(cd "$src" && CARGO_TARGET_DIR="$work/target" cargo build --release -p observe --quiet)

mkdir -p "$out"
cp "$work/target/release/observe" "$out/observe.new"
mv "$out/observe.new" "$out/observe"
(cd "$work" && tar -czf "$out/source.tar.gz.new" rust)
mv "$out/source.tar.gz.new" "$out/source.tar.gz"
{
  echo "built:  $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "from:   $origin"
  echo "sha256: $(shasum -a 256 "$out/observe" | cut -d' ' -f1)"
} > "$out/SOURCE"
echo "rustc:  $(rustc --version)" >> "$out/SOURCE"
# A copy outside the repo, for git worktrees (observe looks there when rust/reference is missing).
mkdir -p "$HOME/treachery-native-ref"
# A new file, then a rename: macOS kills a signed binary that was overwritten in place ("Killed: 9").
cp "$out/observe" "$HOME/treachery-native-ref/observe.new"
mv "$HOME/treachery-native-ref/observe.new" "$HOME/treachery-native-ref/observe"
cp "$out/SOURCE" "$HOME/treachery-native-ref/"
cat "$out/SOURCE"
