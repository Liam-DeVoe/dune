#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Serve a published client with the fixtures, for looking at a scene by hand.

    scripts/ui/serve.py                      # /tmp/treachery-client-after on port 8765
    scripts/ui/serve.py --before --port 9000

Prints a few scene URLs to open.
"""

import argparse
import sys
from pathlib import Path
from urllib.parse import quote, urlencode

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from server import StaticServer  # noqa: E402

EXAMPLES = [
    ("Fremen move, source selected", {"fixture": "move", "faction": "fremen", "steps": "click:False Wall South (East Sector)"}),
    ("Fremen move, nothing selected", {"fixture": "move", "faction": "fremen"}),
    ("Emperor shipment", {"fixture": "ship", "faction": "emperor"}),
    ("Guild shipment, site to site", {"fixture": "ship-guild", "faction": "guild"}),
]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--before", action="store_true", help="serve /tmp/treachery-client-before")
    ap.add_argument("--client", type=Path, help="a published client directory")
    ap.add_argument("--port", type=int, default=8765)
    args = ap.parse_args()
    client = args.client or Path("/tmp/treachery-client-before" if args.before else "/tmp/treachery-client-after")
    try:
        server = StaticServer(client, port=args.port).start()
    except FileNotFoundError as e:
        sys.exit(str(e))
    print(f"serving {client} at {server.url}\n")
    for title, params in EXAMPLES:
        print(f"{title}\n  {server.url}/scene?{urlencode(params, quote_via=quote)}\n")
    print("Ctrl-C stops it.")
    try:
        server._thread.join()
    except KeyboardInterrupt:
        server.stop()


if __name__ == "__main__":
    main()
