#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["playwright==1.58.0"]
# ///
"""Take design screenshots of a published client.

    scripts/ui/shots.py ship worm-ride       # after shots, the default
    scripts/ui/shots.py --before ship        # from /tmp/treachery-client-before
    scripts/ui/shots.py --gallery            # everything, then design/<area>/gallery.html
    scripts/ui/shots.py --list

Each file in scripts/ui/scenarios/ drives one flow and writes PNGs to
design/<area>/<before|after>/. Scenarios run in parallel, each in its own
browser, against one static server that serves the published client and
scripts/fixtures. scripts/README.md explains the workflow and how to write
a scenario.
"""

import argparse
import importlib.util
import os
import queue
import sys
import threading
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from gallery import write_gallery  # noqa: E402
from harness import REPO, Log, Run, relative  # noqa: E402
from playwright.sync_api import sync_playwright  # noqa: E402
from server import StaticServer  # noqa: E402

SCENARIOS = HERE / "scenarios"


def scenario_names() -> list[str]:
    return sorted(p.stem for p in SCENARIOS.glob("*.py"))


def load(name: str):
    path = SCENARIOS / f"{name}.py"
    if not path.exists():
        sys.exit(f"no scenario {name!r}; try one of: {', '.join(scenario_names())}")
    spec = importlib.util.spec_from_file_location(f"scenarios.{name}", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    for attr in ("AREA", "run"):
        if not hasattr(module, attr):
            sys.exit(f"{relative(path)} must define {attr}")
    return module


@dataclass
class Result:
    name: str
    seconds: float = 0
    error: str | None = None
    console_errors: list[str] = field(default_factory=list)


def worker(
    jobs: queue.Queue, results: list[Result], args, url: str, mode: str, echo
) -> None:
    """Takes scenarios off the queue and runs each in its own page of one browser."""
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=not args.headed)
        try:
            while True:
                try:
                    name = jobs.get_nowait()
                except queue.Empty:
                    return
                module = load(name)
                out = args.out or REPO / "design" / module.AREA / mode
                log = Log(name, echo)
                log(f"({mode}) -> {relative(out)}")
                run = Run(mode=mode, out=out, url=url, browser=browser, log=log)
                result = Result(name)
                started = time.monotonic()
                try:
                    module.run(run)
                except Exception as e:
                    result.error = traceback.format_exc()
                    log(f"FAILED: {type(e).__name__}: {(str(e).strip().splitlines() or [''])[0]}")
                result.seconds = time.monotonic() - started
                result.console_errors = run.errors
                results.append(result)
        finally:
            browser.close()


def summarise(results: list[Result], total: float) -> bool:
    print()
    print(f"{'scenario':14} {'time':>6}  result")
    for r in sorted(results, key=lambda r: r.name):
        status = "failed" if r.error else "ok"
        if r.console_errors:
            status += f", {len(r.console_errors)} console error{'s' if len(r.console_errors) > 1 else ''}"
        print(f"{r.name:14} {r.seconds:5.1f}s  {status}")
    print(f"{'total':14} {total:5.1f}s")
    for r in sorted(results, key=lambda r: r.name):
        for line in r.console_errors:
            print(f"{r.name:14} console: {' '.join(line.split())[:300]}")
        if r.error:
            print(f"\n{r.name} traceback:\n{r.error}")
    return not any(r.error for r in results)


def main() -> None:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "scenarios", nargs="*", help="scenario names; all of them if none given"
    )
    ap.add_argument(
        "--before",
        action="store_true",
        help="take the before shots, by default from /tmp/treachery-client-before",
    )
    ap.add_argument(
        "--client",
        type=Path,
        help="a published client to serve (default: /tmp/treachery-client-<before|after>)",
    )
    ap.add_argument(
        "--url",
        default=os.environ.get("TREACHERY_URL"),
        help="use a server that is already running instead of --client; it must serve "
        "the scene page and /fixtures/<name>.json (default: $TREACHERY_URL)",
    )
    ap.add_argument(
        "--out", type=Path, help="write PNGs here instead of design/<area>/<mode>"
    )
    ap.add_argument(
        "--workers", type=int, default=4, help="scenarios to run at once (default 4)"
    )
    ap.add_argument(
        "--gallery",
        action="store_true",
        help="afterwards, write design/<area>/gallery.html for each area touched",
    )
    ap.add_argument("--headed", action="store_true", help="show the browsers")
    ap.add_argument("--list", action="store_true", help="list scenarios and exit")
    args = ap.parse_args()

    if args.list:
        for name in scenario_names():
            print(f"{name:16} {(load(name).__doc__ or '').strip().splitlines()[0]}")
        return

    mode = "before" if args.before else "after"
    names = args.scenarios or scenario_names()
    for name in names:
        load(name)  # a typo should fail before any browser starts

    server = None
    if args.url:
        url = args.url.rstrip("/")
    else:
        client = args.client or Path(f"/tmp/treachery-client-{mode}")
        try:
            server = StaticServer(client).start()
        except FileNotFoundError as e:
            sys.exit(f"{e}\n(or run against a live server with --url)")
        url = server.url
        print(f"serving {client} at {url}")

    jobs: queue.Queue = queue.Queue()
    for name in names:
        jobs.put(name)
    results: list[Result] = []
    lock = threading.Lock()

    def echo(line: str) -> None:
        with lock:
            print(line, flush=True)

    started = time.monotonic()
    threads = [
        threading.Thread(
            target=worker, args=(jobs, results, args, url, mode, echo), daemon=True
        )
        for _ in range(max(1, min(args.workers, len(names))))
    ]
    try:
        for t in threads:
            t.start()
        for t in threads:
            t.join()
    finally:
        if server:
            server.stop()
    ok = summarise(results, time.monotonic() - started)

    if args.gallery and not args.out:
        for area in sorted({load(name).AREA for name in names}):
            path = write_gallery(REPO / "design" / area)
            print(f"wrote {relative(path)}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
