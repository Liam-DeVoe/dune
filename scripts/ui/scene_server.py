"""The Rust server with the dev scene endpoint, for scripted comparisons whose steps make the web
client query or submit (client-plan.md, "Switching to the dev scene endpoint").

The saved views under /views/ can't answer a draft query or take an event, so a form that asks
the server while it is filled in (a battle plan's leader, Clairvoyance's question) or a confirm
button needs a server. `c.scene(fixture, faction, server=True)` in a comparison opens the web side
on this server instead: the same /scene/<fixture>/<Faction> route, which then finds POST /dev/scene
and plays the scene over the WebSocket. The C# side is unchanged; its scene page runs the events
in the browser.

    with SceneServer(web_dist, tag="battle-plan") as url:
        ...  # url is "http://127.0.0.1:<port>"

The server's log is in /tmp/treachery-compare/scene-server/<pid>/<tag>.log, one directory per run.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import threading
from pathlib import Path

from lobby_servers import RUST, RUST_SERVER, free_port, wait_for

WORK = Path("/tmp/treachery-compare/scene-server") / str(os.getpid())
FIXTURES = RUST.parent / "scripts" / "fixtures"

_built = False
_build_lock = threading.Lock()


def build_server() -> None:
    """Builds the Rust server once per run (a no-op when it is current)."""
    global _built
    with _build_lock:
        if _built:
            return
        WORK.mkdir(parents=True, exist_ok=True)
        log = WORK / "build.log"
        p = subprocess.run(["cargo", "build", "--release", "-q", "-p", "treachery-server"], cwd=RUST,
                           stdout=log.open("w"), stderr=subprocess.STDOUT)
        if p.returncode != 0:
            raise RuntimeError(f"building the Rust server failed; log in {log}:\n{log.read_text()[-3000:]}")
        _built = True


class SceneServer:
    """The Rust server on an empty database with dev scenes on, serving the web build. `clock` (an
    ISO 8601 time) stops the server's clock there (TREACHERY_DEV_CLOCK, rust/server/src/settings.rs), so
    events submitted to a scene carry the same time as the pages' fixed clock."""

    def __init__(self, web_dist: Path, tag: str = "run", clock: str | None = None):
        self.web_dist, self.tag, self.clock = web_dist, tag, clock
        self.proc: subprocess.Popen | None = None

    def __enter__(self) -> str:
        build_server()
        work = WORK / self.tag
        if work.exists():
            shutil.rmtree(work)
        work.mkdir(parents=True)
        port = free_port()
        env = {**os.environ, "TREACHERY_WEB_ROOT": str(self.web_dist), "TREACHERY_DEV_SCENE": "true",
               "TREACHERY_DEV_FIXTURES": str(FIXTURES), "TREACHERY_LISTEN": f"127.0.0.1:{port}", "TREACHERY_ADMIN": "admin",
               "TREACHERY_DATABASE": str(work / "scene.db")}
        if self.clock:
            env["TREACHERY_DEV_CLOCK"] = self.clock
        log = WORK / f"{self.tag}.log"
        self.proc = subprocess.Popen([str(RUST_SERVER)], cwd=work, env=env, stdout=log.open("w"), stderr=subprocess.STDOUT)
        url = f"http://127.0.0.1:{port}"
        wait_for(url + "/", self.proc, log)
        return url

    def __exit__(self, *exc) -> None:
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
