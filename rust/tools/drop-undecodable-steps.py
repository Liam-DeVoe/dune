#!/usr/bin/env python3
"""Marks steps of recorded test games as refused, for events the engine no longer decodes.

The recorded games under rust/testdata and the crates' tests/ directories come from builds where
any number was a value of every enum. Since rewrite-plan.md change 2d the engine refuses an event
holding a number that names no member, so a test replaying the game would stop at it.
view/tests/trace_events_decode.rs lists such steps as `PATH line N`. This script sets each
listed step's `result` to "rejected" and adds a `note`, so the replays skip it.

Only use it for events that changed nothing in the game when they were accepted (the test that
replays the game must still reach what it reached before).

    rust/tools/drop-undecodable-steps.py rust/view/tests/data/other/fuzz-1-00070.jsonl.gz:44 ...
"""

import gzip
import json
import sys
from collections import defaultdict


def main():
    lines_by_file = defaultdict(set)
    for arg in sys.argv[1:]:
        path, line = arg.rsplit(":", 1)
        lines_by_file[path].add(int(line))
    for path, numbers in lines_by_file.items():
        with gzip.open(path, "rt") as f:
            lines = f.read().splitlines()
        for n in numbers:
            step = json.loads(lines[n - 1])
            assert step["kind"] == "step" and step["result"] == "accepted", f"{path}:{n} is not an accepted step"
            step["result"] = "rejected"
            step["note"] = "accepted when recorded; refused since rewrite-plan.md 2d because it names no enum member"
            lines[n - 1] = json.dumps(step, separators=(",", ":"), ensure_ascii=False)
        with gzip.open(path, "wt") as f:
            f.write("\n".join(lines) + "\n")
        print(f"{path}: {len(numbers)} steps")


if __name__ == "__main__":
    main()
