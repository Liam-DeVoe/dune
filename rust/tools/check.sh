#!/bin/bash
# Runs the whole Rust test suite and prints one line per test file.
# Every agent runs this before reporting a change as done (PORTING.md, "Before you report done").
#
#   rust/tools/check.sh            # all workspace tests, release profile
#   rust/tools/check.sh -p observe # extra arguments go to cargo test
#
# Tests marked #[ignore] (none of which track open review items at the moment) are
# skipped, as cargo does by default. Exit status is 0 only if everything compiled
# and every test passed.

cd "$(dirname "$0")/.." || exit 2

log=$(mktemp -t check-sh.XXXXXX)
trap 'rm -f "$log"' EXIT

cargo test --workspace --release --no-fail-fast "$@" >"$log" 2>&1
status=$?

# Pair each "Running <file>" / "Doc-tests <crate>" line with the "test result" line after it.
awk '
  /^ *Running / {
    name = $2
    # Unit tests: name them after the crate, from target/release/deps/<crate>-<hash>
    if (name == "unittests") { d = $NF; sub(/^.*\//, "", d); sub(/-[0-9a-f]+\)$/, "", d); name = "unittests " d }
    next
  }
  /^ *Doc-tests / { name = "doc-tests " $2; next }
  /^test result:/ {
    sub(/^test result: /, "")
    split($0, parts, ";")
    printf "%-55s %s;%s;%s\n", name, parts[1], parts[2], parts[3]
    name = ""
  }
  /^test .* \.\.\. FAILED$/ { failed[++n] = $2 }
  END { for (i = 1; i <= n; i++) print "    FAILED " failed[i] }
' "$log"

# Compile errors (test failures are listed above).
if grep -q "^error\[E[0-9]*\]\|^error: could not compile" "$log"; then
  echo
  echo "compile errors:"
  grep -A8 "^error\[E[0-9]*\]" "$log" | head -60
  grep "^error: could not compile" "$log"
fi

if [ $status -eq 0 ]; then
  echo "check.sh: all tests passed"
else
  echo "check.sh: FAILED (cargo exit $status)"
fi
exit $status
