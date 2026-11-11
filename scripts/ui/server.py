"""A static server for a published client, so screenshots need no game server.

It serves a published client's wwwroot, answers /scene (any path without a
file extension) with index.html so the Blazor router can take over, and
serves /fixtures/<name>.json from scripts/fixtures. Runs on a thread; stop()
shuts it down.

    with StaticServer(Path("/tmp/treachery-client-after")) as server:
        page.goto(server.url + "/scene?fixture=ship&faction=emperor")
"""

from __future__ import annotations

import sys
import threading
from functools import partial
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"

# Python's guesses are stale or missing for some of what a Blazor publish contains.
MIME = {
    ".wasm": "application/wasm",
    ".js": "text/javascript",
    ".mjs": "text/javascript",
    ".json": "application/json",
    ".svg": "image/svg+xml",
    ".css": "text/css",
    ".woff2": "font/woff2",
    ".woff": "font/woff",
    ".ttf": "font/ttf",
    ".gif": "image/gif",
    ".png": "image/png",
    ".ico": "image/x-icon",
    ".dat": "application/octet-stream",
    ".blat": "application/octet-stream",
    ".dll": "application/octet-stream",
    ".pdb": "application/octet-stream",
}


class Handler(SimpleHTTPRequestHandler):
    extensions_map = {**SimpleHTTPRequestHandler.extensions_map, **MIME}
    protocol_version = "HTTP/1.1"  # keep connections open: a client start is a few hundred requests

    def __init__(self, *args, wwwroot: Path, **kwargs):
        self.wwwroot = wwwroot
        super().__init__(*args, directory=str(wwwroot), **kwargs)

    def translate_path(self, path: str) -> str:
        route = urlsplit(path).path
        if route.startswith("/fixtures/") and route.endswith(".json"):
            return str(FIXTURES / route.removeprefix("/fixtures/"))
        local = Path(super().translate_path(path))
        # A route like /scene has no file behind it: the app's index.html handles it.
        if not local.exists() and not Path(route).suffix:
            return str(self.wwwroot / "index.html")
        return str(local)

    def end_headers(self) -> None:
        # Fixtures and index.html change between runs; the framework files are content-hashed.
        if self.path.startswith("/fixtures/") or not Path(urlsplit(self.path).path).suffix:
            self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, format: str, *args) -> None:
        pass  # a 404 is reported through log_error; successful requests are noise

    def log_error(self, format: str, *args) -> None:
        # A missing fixture or a stale publish shows up here; keep it on one line.
        if args and args[0] == HTTPStatus.NOT_FOUND:
            print(f"static server: 404 {self.path}")


class Server(ThreadingHTTPServer):
    # Several browsers starting at once open more connections than the default backlog of 5 holds.
    request_queue_size = 256
    daemon_threads = True

    def handle_error(self, request, client_address) -> None:
        # A browser closing a kept-alive connection is not worth a traceback.
        if isinstance(sys.exc_info()[1], (ConnectionResetError, BrokenPipeError)):
            return
        super().handle_error(request, client_address)


class StaticServer:
    """Serves a published client from a free port on 127.0.0.1 until stopped."""

    def __init__(self, client: Path, port: int = 0):
        self.wwwroot = client / "wwwroot"
        if not (self.wwwroot / "index.html").exists():
            raise FileNotFoundError(f"{self.wwwroot}/index.html; publish the client with scripts/publish-client.sh")
        self._httpd = Server(("127.0.0.1", port), partial(Handler, wwwroot=self.wwwroot))
        self._thread = threading.Thread(target=self._httpd.serve_forever, daemon=True)

    @property
    def url(self) -> str:
        host, port = self._httpd.server_address[:2]
        return f"http://{host}:{port}"

    def start(self) -> "StaticServer":
        self._thread.start()
        return self

    def stop(self) -> None:
        self._httpd.shutdown()
        self._httpd.server_close()

    def __enter__(self) -> "StaticServer":
        return self.start()

    def __exit__(self, *exc) -> None:
        self.stop()
