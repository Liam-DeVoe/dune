#!/bin/bash
# Runs a CPU-heavy command (observe live, screenshot runs) only when a slot
# is free, so agents don't oversubscribe the machine's 8 cores.
#
#   rust/tools/heavy.sh ./target/release/observe live --mode fuzz --seed 5 --games 100 --jobs 4
#
# HEAVY_SLOTS sets the number of slots (default 2). A slot whose process has
# died is reclaimed automatically.
#
# HEAVY_MEM_MB (default 3000) caps the memory of each process the command
# starts. A process over the cap is killed, and the run exits with status 137.
# The cap uses top's MEM column, which includes compressed and swapped memory.

slots=${HEAVY_SLOTS:-2}
mem_mb=${HEAVY_MEM_MB:-3000}

descendants() {
  local kids
  kids=$(pgrep -P "$1")
  for k in $kids; do
    echo "$k"
    descendants "$k"
  done
}

# Prints the pid of the first descendant of $1 using more than $mem_mb MB.
over_limit() {
  local pids args
  pids=$(descendants "$1")
  [ -z "$pids" ] && return
  args=$(for p in $pids; do printf -- '-pid %s ' "$p"; done)
  top -l 1 $args -stats pid,mem 2>/dev/null | awk -v lim="$mem_mb" '
    $1 ~ /^[0-9]+$/ {
      m = $2; v = m + 0
      if (m ~ /G/) v *= 1024; else if (m ~ /K/) v /= 1024; else if (m ~ /B$/ && m !~ /[KMG]/) v /= 1048576
      if (v > lim) { print $1; exit }
    }'
}
dir=/tmp/treachery-heavy-slots
mkdir -p "$dir"

while true; do
  for i in $(seq 1 "$slots"); do
    slot="$dir/slot$i"
    if mkdir "$slot" 2>/dev/null; then
      echo $$ > "$slot/pid"
      trap 'rm -rf "$slot"' EXIT
      echo "heavy.sh: running in slot $i" >&2
      # observe live takes a slot itself unless this is set (it would otherwise wait for a second one).
      export TREACHERY_HEAVY_SLOT=$i
      "$@" &
      child=$!
      while kill -0 "$child" 2>/dev/null; do
        big=$(over_limit $$)
        if [ -n "$big" ]; then
          echo "heavy.sh: process $big is over ${mem_mb} MB, killing the run" >&2
          kill -9 "$big" 2>/dev/null
          pkill -9 -P "$child" 2>/dev/null
          kill -9 "$child" 2>/dev/null
          wait "$child" 2>/dev/null
          exit 137
        fi
        sleep 15
      done
      wait "$child"
      exit $?
    fi
    pid=$(cat "$slot/pid" 2>/dev/null)
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      rm -rf "$slot"
    fi
  done
  sleep 10
done
