#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# ///
"""Write design/<area>/gallery.html: every shot in before/ and after/ side by side.

    scripts/ui/gallery.py map-ux                 # writes design/map-ux/gallery.html
    scripts/ui/gallery.py map-ux --out /tmp/g.html

shots.py --gallery does the same for each area it touched. A shot's zoom
(<name>-zoom.png) sits under its full shot. The page is plain HTML with
image paths relative to itself, so it works from the repo checkout.
"""

from __future__ import annotations

import argparse
import html
import os
import sys
from pathlib import Path

STYLE = """
body{margin:0;background:#1d0c00;color:#fcf9d1;font-family:"Alegreya Sans",Candara,sans-serif;padding:32px 48px}
h1{font-weight:400;font-size:36px;margin:0 0 4px} p{color:#b3915f;max-width:80ch}
h3{font-weight:600;font-size:18px;margin:36px 0 10px;color:#f3d987;font-family:ui-monospace,Menlo,monospace}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:16px;align-items:start}
.lbl{font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:#a68d63;margin:0 0 6px}
.shot{border:1px solid rgba(255,255,255,.12);border-radius:6px;overflow:hidden;background:#000}
.shot img{width:100%;display:block} .zoom{margin-top:10px;width:60%}
.missing{border:1px dashed rgba(255,255,255,.2);border-radius:6px;padding:24px;color:#a68d63;font-size:14px}
"""


def names(directory: Path) -> set[str]:
    """Shot names in a directory, without .png and without the -zoom suffix."""
    return {p.stem.removesuffix("-zoom") for p in directory.glob("*.png")}


def cell(directory: Path, name: str, label: str, base: Path) -> str:
    """One column of a pair: the full shot and, if there is one, its zoom under it.

    Image paths are written relative to base, the directory the HTML lives in.
    """
    parts = [f'<p class="lbl">{label}</p>']
    full, zoom = directory / f"{name}.png", directory / f"{name}-zoom.png"
    if not full.exists() and not zoom.exists():
        parts.append('<div class="missing">no shot</div>')
    for path, cls, alt in ((full, "shot", label), (zoom, "shot zoom", f"{label} zoom")):
        if path.exists():
            src = html.escape(os.path.relpath(path, base))
            parts.append(f'<div class="{cls}"><img src="{src}" alt="{html.escape(name)} {alt}"></div>')
    return f"<div>{''.join(parts)}</div>"


def write_gallery(area: Path, out: Path | None = None) -> Path:
    """Write the gallery for design/<area> and return its path."""
    before, after = area / "before", area / "after"
    all_names = sorted(names(before) | names(after))
    if not all_names:
        raise FileNotFoundError(f"no PNGs under {before} or {after}")
    out = out or area / "gallery.html"
    base = out.resolve().parent
    sections = "".join(
        f'<h3 id="{html.escape(n)}">{html.escape(n)}</h3>'
        f'<div class="pair">{cell(before, n, "Before", base)}{cell(after, n, "After", base)}</div>'
        for n in all_names)
    title = f"{area.name}: before and after"
    page = (f'<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>{html.escape(title)}</title>'
            f"<style>{STYLE}</style></head><body><h1>{html.escape(title)}</h1>"
            f"<p>Every shot under before/ and after/, paired by name. {len(all_names)} shots. "
            f"Written by scripts/ui/gallery.py.</p>{sections}</body></html>\n")
    out.write_text(page)
    return out


def main() -> None:
    repo = Path(__file__).resolve().parent.parent.parent
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("area", help="a directory name under design/, such as map-ux")
    ap.add_argument("--out", type=Path, help="write the HTML here instead of design/<area>/gallery.html")
    args = ap.parse_args()
    area = repo / "design" / args.area
    if not area.is_dir():
        sys.exit(f"no such area: {area}")
    out = write_gallery(area, args.out)
    print(f"wrote {os.path.relpath(out)}")


if __name__ == "__main__":
    main()
