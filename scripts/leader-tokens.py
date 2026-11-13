# /// script
# dependencies = ["pillow==11.3.0"]
# ///
"""Draws each leader's strength in a badge on its token.

The original token art (treachery-online/Client/wwwroot/art/person*.png and .gif) has the
strength printed on the disc beside the portrait. design/leader-tokens/clean/
holds the same tokens with that number retouched out by hand, one
leader<id>.png per leader. This shrinks each clean token a little and draws
the strength in ink on a parchment disc centred on the ring at the top right.
The disc is big enough to stick out past the token's square, so the image is
CANVAS times the token's square, with the token in the middle. The client
scales these images up by CANVAS (see .leader-art in _leader.scss), so the
token draws at its usual size and the disc overhangs it. The result goes to
web/assets/art/leader<id>.png, which rust/skin and web/public/skin/default.json
point at. Those URLs end in ?v=<n> so browsers don't keep showing old tokens:
bump it in both whenever the tokens are redrawn.

    uv run scripts/leader-tokens.py            # writes the tokens
    uv run scripts/leader-tokens.py --sheet    # also writes /tmp/leader-tokens.png to review
"""

import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
ROOT = REPO / "treachery-online"
CLEAN = REPO / "design" / "leader-tokens" / "clean"
ART = REPO / "web" / "assets" / "art"
DIGITS = REPO / "web" / "assets" / "Advokat.ttf"
LETTERS = REPO / "web" / "assets" / "Copperplate.ttf"  # for Zoal's X, which Advokat draws oddly

TOKEN_SCALE = 0.86      # the token's diameter in the new image, as a fraction of its side
BADGE = 0.44            # the badge's diameter, same units
BADGE_OUTLINE = 0.025   # its dark edge
# The image's side, in the same units: room for the badge plus a hair. Keep in
# step with $leader-art-canvas in web/styles/components/_leader.scss.
CANVAS = 1.068
INK = (18, 8, 0, 255)
PARCHMENT = (252, 249, 209, 255)


def leaders():
    """(id, strength text, faction colour) for every leader."""
    manager = (ROOT / "Shared" / "LeaderManager.cs").read_text()
    skin = (ROOT / "Shared" / "DefaultSkin.cs").read_text()
    block = skin[skin.index("FactionColor = "):]
    colours = dict(re.findall(r'\[Faction\.(\w+)\] = "(#[0-9A-Fa-f]{6})"', block[: block.index("};")]))
    for m in re.finditer(r"new Leader\((\d+)\) \{([^}]*)\}", manager):
        id, body = m.groups()
        value = re.search(r"Value = (\d+)", body)
        faction = re.search(r"Faction = Faction\.(\w+)", body)
        yield id, value.group(1) if value else "X", colours[faction.group(1)]


def badge(size, text):
    """The strength on a disc on the ring, up and to the right of the token's centre.
    size is the side of the token's square; the result is CANVAS times that."""
    u = 4 * size  # the token's square, drawn large and scaled down for smooth edges
    s = round(CANVAS * size) * 4
    out = Image.new("RGBA", (s, s))
    d = ImageDraw.Draw(out)
    r = BADGE * u / 2
    off = 0.707 * TOKEN_SCALE * u / 2
    cx, cy = s / 2 + off, s / 2 - off
    assert cx + r <= s and cy - r >= 0, "the badge doesn't fit; raise CANVAS"
    edge = BADGE_OUTLINE * u
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=INK)
    d.ellipse((cx - r + edge, cy - r + edge, cx + r - edge, cy + r - edge), fill=PARCHMENT)
    digit = text.isdigit()
    font = ImageFont.truetype(str(DIGITS if digit else LETTERS), int(BADGE * u * (0.74 if digit else 0.56)))
    d.text((cx, cy), text, font=font, fill=INK, anchor="mm")
    return out.resize((s // 4,) * 2, Image.Resampling.LANCZOS)


def main():
    rows = list(leaders())
    missing = [id for id, _, _ in rows if not (CLEAN / f"leader{id}.png").exists()]
    if missing:
        sys.exit(f"no clean token for {', '.join(missing)} in {CLEAN}")

    for id, value, _ in rows:
        clean = Image.open(CLEAN / f"leader{id}.png").convert("RGBA")
        size = max(clean.size)
        token = clean.resize((round(TOKEN_SCALE * size),) * 2, Image.Resampling.LANCZOS)
        out = Image.new("RGBA", (round(CANVAS * size),) * 2)
        offset = (out.width - token.width) // 2
        out.paste(token, (offset, offset), token)
        out.alpha_composite(badge(size, value))
        out.save(ART / f"leader{id}.png")
    print(f"wrote {len(rows)} tokens to {ART}")

    if "--sheet" in sys.argv:
        tiles = [Image.open(ART / f"leader{id}.png").resize((150, 150)) for id, _, _ in rows]
        sheet = Image.new("RGBA", (150 * 12, 150 * ((len(tiles) + 11) // 12)), (40, 20, 5, 255))
        for i, t in enumerate(tiles):
            sheet.paste(t, ((i % 12) * 150, (i // 12) * 150), t)
        sheet.save("/tmp/leader-tokens.png")


if __name__ == "__main__":
    main()
