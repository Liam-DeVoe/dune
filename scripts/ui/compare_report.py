"""Writes the comparison galleries from compare.py's results.

Each run writes its own directory, design/compare/runs/<time>-<pid> by default:

    <run>/gallery.html                        one matrix per region and viewport, this run only
    <run>/<fixture>/<Faction>/index.html      the shots, diffs and DOM diffs of a scene
    <run>/scripts/<script>/<shot>/index.html  the same for a shot of a scripted comparison
    <run>/results.json                        this run's results, for scripts

The shared design/compare/gallery.html and results.json hold the latest result of every scene
across runs. They link into the run directories. A run that compared every scene replaces them
all, and a scripted comparison that ran replaces all of its earlier shots. Several runs can
finish at once: the shared files are merged under a lock and written by rename, so a reader
never sees half a file. Old run directories are deleted (see prune), except those the shared
gallery still points to.

Image paths are relative, so the pages work from the checkout.
"""

from __future__ import annotations

import fcntl
import html
import json
import os
import shutil
import time
from contextlib import contextmanager
from pathlib import Path

# Pruning: the newest KEEP_RUNS run directories are always kept. An older one is deleted once
# the shared gallery no longer points into it, that is, once later runs have compared all of its
# scenes again. So the runs on disk are at most KEEP_RUNS plus one result per scene.
KEEP_RUNS = 20
RUNS = "runs"

STATUS_TEXT = {
    "match": "match",
    "differ": "differs",
    "missing": "only in C#",
    "extra": "only in web",
    "absent": "in neither",
    "error": "scene failed",
}

# Colours from design/design-language.html.
STYLE = """
:root{--ink-0:#120800;--ink-1:#1d0c00;--ink-2:#2a1607;--ink-3:#3b2310;--parchment:#fcf9d1;--text-2:#d9c79a;--text-3:#a68d63;
--gold:#e7bf3c;--sand:#f3d987;--good:#63842e;--storm:#c33232;--spice:#e8762a;--blue:#4f7bb3}
body{margin:0;background:var(--ink-1);color:var(--parchment);font-family:"Alegreya Sans",Candara,Calibri,sans-serif;font-size:16px;line-height:1.45;padding:32px 48px}
a{color:var(--gold);text-decoration:none} a:hover{text-decoration:underline}
h1{font-weight:400;font-size:34px;margin:0 0 6px} h2{font-weight:400;font-size:24px;color:var(--gold);margin:40px 0 8px}
h3{font-weight:600;font-size:16px;color:var(--sand);margin:28px 0 8px;font-family:ui-monospace,Menlo,monospace}
p,li{color:var(--text-2);max-width:90ch} code{font-family:ui-monospace,Menlo,monospace;font-size:.85em;color:var(--sand)}
.legend span{display:inline-flex;align-items:center;gap:6px;margin-right:18px;color:var(--text-2);font-size:14px}
.sq{display:inline-block;width:14px;height:14px;border-radius:3px;vertical-align:middle}
.match{background:var(--good)} .differ{background:var(--storm)} .missing{background:var(--spice)} .extra{background:var(--blue)}
.absent{background:var(--ink-3)} .error{background:#000;outline:1px solid var(--storm)} .none{background:transparent}
.matrices{display:grid;grid-template-columns:repeat(auto-fill,minmax(460px,1fr));gap:24px 40px}
table.m{border-collapse:collapse;font-size:12px} table.m th{font-weight:400;color:var(--text-3);padding:1px 4px;text-align:left;white-space:nowrap}
table.m th.f{writing-mode:vertical-rl;transform:rotate(180deg);height:64px;text-align:left;padding:2px 0}
table.m td{padding:1px} table.m td a{display:block;width:16px;height:16px;border-radius:3px}
table.m caption{text-align:left;font-size:15px;color:var(--sand);padding-bottom:6px;caption-side:top}
table.m caption b{color:var(--parchment)}
.shots{display:grid;grid-template-columns:1fr 1fr 1fr;gap:12px;align-items:start}
.lbl{font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:var(--text-3);margin:0 0 4px}
.shot{border:1px solid rgba(255,255,255,.12);border-radius:6px;overflow:hidden;background:#000;max-height:900px;overflow-y:auto}
.shot img{width:100%;display:block}
.nothing{border:1px dashed rgba(255,255,255,.2);border-radius:6px;padding:16px;color:var(--text-3);font-size:14px}
pre{background:var(--ink-0);border-radius:6px;padding:10px 14px;font-size:12px;overflow:auto;max-height:520px;color:var(--text-2)}
pre .add{color:#9fc96a} pre .del{color:#e88a7a}
.err{color:#e88a7a} details summary{cursor:pointer;color:var(--text-3);font-size:14px}
.stat{font-size:14px;color:var(--text-2)}
"""

FACTION_ORDER = ["Atreides", "Harkonnen", "Fremen", "Emperor", "Guild", "BeneGesserit", "Ixian", "Tleilaxu", "Choam", "Richese", "Ecaz", "Moritani", "Observer"]


def esc(s) -> str:
    return html.escape(str(s))


def page(title: str, body: str) -> str:
    return (f'<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>{esc(title)}</title>'
            f"<style>{STYLE}</style></head><body>{body}</body></html>\n")


def legend() -> str:
    return '<p class="legend">' + "".join(f'<span><i class="sq {s}"></i>{t}</span>' for s, t in STATUS_TEXT.items()) + "</p>"


def merge(old: dict | None, data: dict) -> dict:
    """data with the scenes of old that data did not compare. A run of every scene replaces all."""
    if not data["partial"] or not old:
        return data
    fresh = {(s["fixture"], s["faction"]) for s in data["scenes"] if not s.get("script")}
    kept = [s for s in old.get("scenes", [])
            if (s.get("script") or (s["fixture"], s["faction"]) not in fresh) and s.get("script") not in data["scripts"]]
    data = {**data, "scenes": sorted(kept + data["scenes"], key=lambda s: (s.get("script") or "", s["fixture"], s["faction"], s.get("step", "")))}
    data["fixtureErrors"] = {**old.get("fixtureErrors", {}), **data["fixtureErrors"]}
    return data


def write_atomic(path: Path, text: str) -> None:
    tmp = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    tmp.write_text(text)
    os.replace(tmp, path)


@contextmanager
def locked(root: Path):
    """Holds root/.lock: one run at a time reads, merges and writes the shared files."""
    root.mkdir(parents=True, exist_ok=True)
    with open(root / ".lock", "w") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def read_results(path: Path) -> dict | None:
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return None


def write_run(run: Path, data: dict) -> Path:
    """This run's results, scene pages and gallery, in its own directory."""
    run.mkdir(parents=True, exist_ok=True)
    write_atomic(run / "results.json", json.dumps(data, indent=1))
    for scene in data["scenes"]:
        if scene.get("dir"):
            write_scene(run, data, scene)
    gallery = run / "gallery.html"
    write_atomic(gallery, write_gallery(data, "this run"))
    return gallery


def publish(root: Path, run: Path, data: dict) -> Path | None:
    """Merges this run into root's shared results.json and gallery.html, then prunes old runs.
    Does nothing for a run outside root (--out elsewhere, --self-check)."""
    if not run.resolve().is_relative_to(root.resolve()):
        return None
    prefix = os.path.relpath(run.resolve(), root.resolve())
    data = {**data, "scenes": [{**s, "dir": f'{prefix}/{s["dir"]}' if s.get("dir") else ""} for s in data["scenes"]]}
    with locked(root):
        merged = merge(read_results(root / "results.json"), data)
        prune(root, merged, keep={prefix.split("/")[1]} if prefix.startswith(RUNS + "/") else set())
        # Drop results whose run directory is gone (pruned, or deleted by hand).
        merged["scenes"] = [s for s in merged["scenes"] if not s.get("dir", "").startswith(RUNS + "/") or (root / s["dir"]).exists()]
        write_atomic(root / "results.json", json.dumps(merged, indent=1))
        gallery = root / "gallery.html"
        write_atomic(gallery, write_gallery(merged, "every run"))
    return gallery


def run_alive(d: Path) -> bool:
    """A run directory still being written: no results.json yet, and its process is running."""
    if (d / "results.json").exists():
        return False
    try:
        os.kill(int(d.name.rsplit("-", 1)[1]), 0)
        return True
    except (IndexError, ValueError):
        return time.time() - d.stat().st_mtime < 24 * 3600  # not named by compare.py: keep a day
    except PermissionError:
        return True
    except OSError:
        return False


def prune(root: Path, merged: dict, keep: set[str]) -> None:
    """Deletes the run directories beyond the newest KEEP_RUNS that the shared results no
    longer point into and that no running compare.py is writing."""
    runs = root / RUNS
    if not runs.is_dir():
        return
    used = {s["dir"].split("/")[1] for s in merged["scenes"] if s.get("dir", "").startswith(RUNS + "/")}
    dirs = sorted((d for d in runs.iterdir() if d.is_dir()), key=lambda d: d.name, reverse=True)
    for d in dirs[KEEP_RUNS:]:
        if d.name in used or d.name in keep or run_alive(d):
            continue
        shutil.rmtree(d, ignore_errors=True)


def write_report(out: Path, data: dict) -> Path:
    """The layout before run directories: results merged into out itself. compare.py versions
    from before runs/ still call this when they finish."""
    out.mkdir(parents=True, exist_ok=True)
    with locked(out):
        merged = merge(read_results(out / "results.json"), data)
        write_atomic(out / "results.json", json.dumps(merged, indent=1))
        for scene in data["scenes"]:  # only this run's: the others' pages are already written
            if scene.get("dir"):
                write_scene(out, data, scene)
        gallery = out / "gallery.html"
        write_atomic(gallery, write_gallery(merged, "every run"))
    return gallery


def label_of(s: dict) -> str:
    return f'{s["script"]}:{s["step"]}' if s.get("script") else f'{s["fixture"]}/{s["faction"]}'


def ok(r: dict) -> bool:
    return r["status"] in ("match", "absent")


def write_gallery(data: dict, scope: str) -> str:
    everything = data["scenes"]
    scenes = [s for s in everything if not s.get("script")]
    scripted = [s for s in everything if s.get("script")]
    fixtures = sorted({s["fixture"] for s in scenes})
    factions = [f for f in FACTION_ORDER if any(s["faction"] == f for s in scenes)]
    by_key = {(s["fixture"], s["faction"]): s for s in scenes}

    parts = ['<h1>Web client against the C# client</h1>',
             f'<p>Every fixture in <code>scripts/fixtures</code>, seen by each of its factions and by an observer, in both clients. '
             f'A region matches when its DOM digests are equal and no more than {data["tolerance"]["pixelShare"]:.2%} of its pixels differ '
             f'by more than {data["tolerance"]["channel"]} in a channel. A region shown by neither client counts as a match. '
             f'Click a square for the shots and the DOM diff. The latest result of each scene from {esc(scope)}; '
             f'written {esc(data["written"])} by <code>just compare</code>.</p>',
             legend()]

    # Counts first: what a client package agent wants to know at a glance.
    parts.append('<h2>Matching scenes per region</h2><table class="m"><tr><th></th>'
                 + "".join(f"<th>{esc(v)}</th>" for v in data["viewports"]) + "</tr>")
    for region in data["regions"]:
        cells = []
        for viewport in data["viewports"]:
            rs = [r for s in scenes for r in s["regions"] if r["region"] == region and r["viewport"] == viewport]
            good = sum(ok(r) for r in rs)
            cells.append(f"<td style='padding:2px 12px'>{good} of {len(rs)}</td>")
        parts.append(f"<tr><th>{esc(region)}</th>{''.join(cells)}</tr>")
    parts.append("</table>")

    failed = [s for s in everything if s.get("error") or s.get("csharp_error") or s.get("web_error")]
    if failed or data["fixtureErrors"]:
        parts.append("<h2>Scenes that could not be compared</h2><ul>")
        for name, err in sorted(data["fixtureErrors"].items()):
            parts.append(f"<li><code>{esc(name)}</code>: the view crate could not replay it: {esc(err)}</li>")
        for s in failed:
            what = ("harness error" if s.get("error") else "C# scene page" if s.get("csharp_error") else "web client")
            msg = (s.get("error") or "").strip().splitlines()[-1:] or [s.get("csharp_error") or s.get("web_error")]
            link = f'<a href="{esc(s["dir"])}/index.html">{esc(label_of(s))}</a>' if s.get("dir") else esc(label_of(s))
            parts.append(f"<li>{link} ({what}): {esc(msg[0])}</li>")
        parts.append("</ul>")

    parts.append("<h2>Masked expected differences</h2><ul>")
    for name, reason in data["masks"].items():
        n = sum(name in s["leaks"] for s in scenes)
        parts.append(f"<li><code>{esc(name)}</code>, in {n} scenes: {esc(reason)}</li>")
    parts.append("</ul>")

    if scripted:
        parts.append('<h2>Scripted comparisons</h2><p>Shots taken after driving both clients through the same clicks '
                     '(<code>scripts/ui/comparisons</code>).</p><table class="m"><tr><th></th>'
                     + "".join(f'<th class="f">{esc(v)} {esc(r)}</th>' for v in data["viewports"] for r in data["regions"]) + "</tr>")
        for s in scripted:
            cells = []
            for v in data["viewports"]:
                for region in data["regions"]:
                    r = next((r for r in s["regions"] if r["region"] == region and r["viewport"] == v), None)
                    cells.append(f'<td><a class="{r["status"]}" title="{esc(STATUS_TEXT[r["status"]])}" href="{esc(s["dir"])}/index.html#{esc(v)}-{esc(region)}"></a></td>'
                                 if r and s.get("dir") else '<td><a class="none"></a></td>')
            parts.append(f'<tr><th>{esc(label_of(s))} <span style="color:var(--text-3)">({esc(s["fixture"])}/{esc(s["faction"])})</span></th>{"".join(cells)}</tr>')
        parts.append("</table>")

    for viewport in data["viewports"]:
        parts.append(f'<h2>{esc(viewport)}</h2><div class="matrices">')
        for region in data["regions"]:
            head = "".join(f'<th class="f">{esc(f)}</th>' for f in factions)
            rows = []
            good = total = 0
            for fx in fixtures:
                cells = []
                for f in factions:
                    s = by_key.get((fx, f))
                    r = s and next((r for r in s["regions"] if r["region"] == region and r["viewport"] == viewport), None)
                    if not r:
                        cells.append('<td><a class="none"></a></td>')
                        continue
                    total += 1
                    good += ok(r)
                    tip = f"{fx}/{f}: {STATUS_TEXT[r['status']]}"
                    if r.get("pixel_share") is not None:
                        tip += f", {r['pixel_share']:.1%} of pixels"
                    href = f' href="{esc(s["dir"])}/index.html#{esc(viewport)}-{esc(region)}"' if s.get("dir") else ""
                    cells.append(f'<td><a class="{r["status"]}" title="{esc(tip)}"{href}></a></td>')
                rows.append(f"<tr><th>{esc(fx)}</th>{''.join(cells)}</tr>")
            parts.append(f'<table class="m"><caption><b>{esc(region)}</b> &nbsp;{good} of {total} match</caption>'
                         f"<tr><th></th>{head}</tr>{''.join(rows)}</table>")
        parts.append("</div>")
    return page("Web client against the C# client", "".join(parts))


def diff_html(diff: str) -> str:
    lines = []
    for line in diff.splitlines():
        cls = "add" if line.startswith("+") and not line.startswith("+++") else "del" if line.startswith("-") and not line.startswith("---") else ""
        lines.append(f'<span class="{cls}">{esc(line)}</span>' if cls else esc(line))
    return "<pre>" + "\n".join(lines) + "</pre>"


def crop_svg(src: str, b: list | None) -> str:
    """The region of a page shot: an SVG whose viewBox is the region, so one shot per client and
    viewport serves every region."""
    if not b:
        return '<div class="nothing">not shown</div>'
    x, y, w, h = b
    return (f'<div class="shot"><svg viewBox="{x} {y} {w} {h}" style="display:block;width:100%">'
            f'<image href="{esc(src)}" x="0" y="0"/></svg></div>')


def write_scene(out: Path, data: dict, s: dict) -> None:
    d = out / s["dir"]
    d.mkdir(parents=True, exist_ok=True)
    key = f'{s["fixture"]}/{s["faction"]}'
    up = "../" * (s["dir"].count("/") + 1)
    title = f'{label_of(s)}, scene {key}' if s.get("script") else key
    parts = [f'<p><a href="{up}gallery.html">All scenes</a></p><h1>{esc(title)}</h1>',
             f'<p>C#: <code>/scene?fixture={esc(s["fixture"])}&amp;faction={esc(s["faction"].lower())}</code>. '
             f'Web: <code>/scene/{esc(key)}</code>.</p>', legend()]
    for what, err in (("Harness", s.get("error")), ("C# scene page", s.get("csharp_error")), ("Web client", s.get("web_error"))):
        if err:
            parts.append(f'<p class="err">{what} failed:</p><pre>{esc(err)}</pre>')
    shown = [n for n in data["masks"] if n in s["leaks"] or any(m["name"] == n for m in s["masks"])]
    if shown:
        parts.append("<p>Masked (painted magenta in both, dropped from both DOM digests):</p><ul>")
        for name in shown:
            n = sum(m["name"] == name for m in s["masks"])
            parts.append(f"<li><code>{esc(name)}</code> ({n} areas on the desktop page): {esc(data['masks'][name])}</li>")
        parts.append("</ul>")
    for client, lines in s.get("console", {}).items():
        if lines:
            parts.append(f"<details><summary>{len(lines)} console errors in the {esc(client)} client</summary>"
                         f"<pre>{esc(chr(10).join(lines[:50]))}</pre></details>")
    for r in s["regions"]:
        anchor = f'{r["viewport"]}-{r["region"]}'
        stat = STATUS_TEXT[r["status"]]
        if r.get("pixel_share") is not None:
            stat += f'; {r["pixel_share"]:.2%} of pixels differ'
        if r.get("size") and r["size"]["csharp"] != r["size"]["web"]:
            stat += f'; size {r["size"]["csharp"][0]}×{r["size"]["csharp"][1]} in C#, {r["size"]["web"][0]}×{r["size"]["web"][1]} in web'
        if r.get("dom_equal") is not None:
            stat += "; DOM equal" if r["dom_equal"] else "; DOM differs"
        parts.append(f'<h3 id="{esc(anchor)}"><i class="sq {r["status"]}"></i> {esc(r["viewport"])} · {esc(r["region"])}</h3>'
                     f'<p class="stat">{esc(stat)}</p>')
        if r["status"] in ("absent", "error"):
            continue
        boxes = r.get("boxes") or {}
        cells = []
        for k, label in (("csharp", "C#"), ("web", "Web")):
            cells.append(f'<div><p class="lbl">{label}</p>{crop_svg(r["viewport"] + "." + k + ".webp", boxes.get(k))}</div>')
        diff = (f'<div class="shot"><img loading="lazy" src="{esc(r["diff"])}" alt="differing pixels"></div>'
                if r.get("diff") else '<div class="nothing">none</div>')
        cells.append(f'<div><p class="lbl">Differing pixels</p>{diff}</div>')
        parts.append(f'<div class="shots">{"".join(cells)}</div>')
        if r.get("dom_diff"):
            parts.append(f"<details><summary>DOM diff, C# (−) against web (+)</summary>{diff_html(r['dom_diff'])}</details>")
    (d / "index.html").write_text(page(key, "".join(parts)))


if __name__ == "__main__":
    # Rewrites the shared gallery from design/compare/results.json, after a change to this file.
    root = Path(__file__).resolve().parent.parent.parent / "design" / "compare"
    with locked(root):
        data = json.loads((root / "results.json").read_text())
        gallery = root / "gallery.html"
        write_atomic(gallery, write_gallery(data, "every run"))
    print(gallery)
