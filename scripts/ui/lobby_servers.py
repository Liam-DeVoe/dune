"""Both game servers, for comparisons of the screens that need one: login, the lobby, game
setup and seating (client-plan.md package 8).

The scene pages need no server, but these screens talk to the server's account and lobby
protocol. So a lobby comparison starts the C# server (treachery-online/Server, which serves the
C# client) and the Rust server (rust/server, serving the web client's build), each on a new,
empty database, and drives the same users through both:

    with LobbyServers(web_dist) as servers:
        servers.urls  # {"csharp": "http://127.0.0.1:...", "web": "http://127.0.0.1:..."}

Each server's log is in /tmp/treachery-compare/lobby/<pid>/<tag>/<client>.log, one directory per
run, so runs at the same time don't share a database.
"""

from __future__ import annotations

import os
import shutil
import socket
import subprocess
import threading
import time
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
CSHARP_SERVER = REPO / "treachery-online" / "Server"
CSHARP_DLL = CSHARP_SERVER / "bin" / "Debug" / "net10.0" / "Treachery.Server.dll"
RUST = REPO / "rust"
RUST_SERVER = RUST / "target" / "release" / "treachery-server"
WORK = Path("/tmp/treachery-compare/lobby") / str(os.getpid())

# Runs the command under /tmp/treachery-build.lock, the lock of scripts/publish-client.sh.
BUILD_LOCKED = (
    'for _ in $(seq 1 600); do mkdir /tmp/treachery-build.lock 2>/dev/null && break; sleep 1; done; '
    "trap 'rmdir /tmp/treachery-build.lock 2>/dev/null' EXIT; \"$@\""
)

_built = False
_build_lock = threading.Lock()


def dotnet_env() -> dict[str, str]:
    env = {**os.environ, "DOTNET_ROOT": os.environ.get("DOTNET_ROOT", str(Path.home() / "dotnet10"))}
    env["PATH"] = f"{env['DOTNET_ROOT']}:{env['PATH']}"
    env.update(DOTNET_CLI_TELEMETRY_OPTOUT="1", DOTNET_NOLOGO="1")
    return env


def build_servers() -> None:
    """Builds the C# server (with its client) and the Rust server, once per run."""
    global _built
    with _build_lock:
        if _built:
            return
        WORK.mkdir(parents=True, exist_ok=True)
        steps = [
            # The server builds the client too, so it takes publish-client.sh's lock: two builds of
            # the same tree at once trample obj/ and bin/.
            ("csharp-server", ["bash", "-c", BUILD_LOCKED, "build", "dotnet", "build", "-v", "q"], CSHARP_SERVER, dotnet_env()),
            ("rust-server", ["cargo", "build", "--release", "-q", "-p", "treachery-server"], RUST, dict(os.environ)),
        ]
        procs = []
        for name, cmd, cwd, env in steps:
            log = WORK / f"build-{name}.log"
            procs.append((name, log, subprocess.Popen(cmd, cwd=cwd, env=env, stdout=log.open("w"), stderr=subprocess.STDOUT)))
        for name, log, p in procs:
            if p.wait() != 0:
                raise RuntimeError(f"building the {name} failed; log in {log}:\n{log.read_text()[-3000:]}")
        _built = True


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wait_for(url: str, proc: subprocess.Popen, log: Path, timeout: float = 90) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"{url}: the server exited; log in {log}:\n{log.read_text()[-3000:]}")
        try:
            with urllib.request.urlopen(url, timeout=2) as r:
                if r.status == 200:
                    return
        except Exception:
            pass
        time.sleep(0.3)
    raise RuntimeError(f"{url}: no answer in {timeout:.0f}s; log in {log}")


class LobbyServers:
    """The C# server and the Rust server on empty databases, stopped when the block ends."""

    def __init__(self, web_dist: Path, tag: str = "run"):
        self.web_dist, self.tag = web_dist, tag
        self.procs: list[subprocess.Popen] = []
        self.urls: dict[str, str] = {}

    def __enter__(self) -> "LobbyServers":
        build_servers()
        work = WORK / self.tag
        if work.exists():
            shutil.rmtree(work)
        work.mkdir(parents=True)
        servers = {
            # Development: the C# server serves the client's static web assets from the build.
            "csharp": (["dotnet", str(CSHARP_DLL)], CSHARP_SERVER, {**dotnet_env(), "ASPNETCORE_ENVIRONMENT": "Development"}),
            "web": ([str(RUST_SERVER)], work, {**os.environ, "TREACHERY_WEB_ROOT": str(self.web_dist)}),
        }
        for client, (cmd, cwd, env) in servers.items():
            port = free_port()
            db = work / (client + ".db")
            if client == "csharp":
                env = {**env, "GameAdminUsername": "admin", "GameEndEmailUsername": "", "ASPNETCORE_URLS": f"http://127.0.0.1:{port}",
                       "ConnectionStrings__TreacheryDatabase": f"Data Source={db}"}
            else:
                env = {**env, "TREACHERY_ADMIN": "admin", "TREACHERY_LISTEN": f"127.0.0.1:{port}", "TREACHERY_DATABASE": str(db)}
            log = work / f"{client}.log"
            proc = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=log.open("w"), stderr=subprocess.STDOUT)
            self.procs.append(proc)
            self.urls[client] = f"http://127.0.0.1:{port}"
            wait_for(self.urls[client] + "/", proc, log)
        return self

    def __exit__(self, *exc) -> None:
        for p in self.procs:
            p.terminate()
        for p in self.procs:
            try:
                p.wait(10)
            except subprocess.TimeoutExpired:
                p.kill()
