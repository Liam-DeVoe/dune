# /// script
# dependencies = ["pillow==11.3.0"]
# ///
"""Draws each leader's strength in a badge on its token.

The shipped token art (Client/wwwroot/art/person*.png and .gif) has the
strength printed on the disc beside the portrait. design/leader-tokens/clean/
holds the same tokens with that number retouched out by hand, one
leader<id>.png per leader. This shrinks each clean token a little and draws
the strength on a disc in the faction's colour, overlapping the ring at the
top right. The result goes to Client/wwwroot/art/leader<id>.png, which
DefaultSkin.cs and skin.json point at.

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
ART = ROOT / "Client" / "wwwroot" / "art"
DIGITS = ROOT / "Client" / "wwwroot" / "Advokat.ttf"
LETTERS = ROOT / "Client" / "wwwroot" / "Copperplate.ttf"  # for Zoal's X, which Advokat draws oddly

TOKEN_SCALE = 0.86      # the token's diameter in the new image, as a fraction of its side
BADGE = 0.36            # the badge's diameter, same units
BADGE_OUTLINE = 0.025   # its dark edge
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


def badge(size, text, colour):
    """The strength on a disc on the ring, up and to the right of the token's centre."""
    s = 4 * size  # drawn large and scaled down, for smooth edges
    out = Image.new("RGBA", (s, s))
    d = ImageDraw.Draw(out)
    r = BADGE * s / 2
    off = 0.707 * TOKEN_SCALE * s / 2
    cx, cy = s / 2 + off, s / 2 - off
    edge = BADGE_OUTLINE * s
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=INK)
    d.ellipse((cx - r + edge, cy - r + edge, cx + r - edge, cy + r - edge), fill=colour)
    digit = text.isdigit()
    font = ImageFont.truetype(str(DIGITS if digit else LETTERS), int(BADGE * s * (0.74 if digit else 0.56)))
    for ox, oy in ((-1, 0), (1, 0), (0, -1), (0, 1), (0, 2)):  # a soft dark edge on the number
        d.text((cx + ox * edge * 0.6, cy + oy * edge * 0.6), text, font=font, fill=(18, 8, 0, 170), anchor="mm")
    d.text((cx, cy), text, font=font, fill=PARCHMENT, anchor="mm")
    return out.resize((size, size), Image.Resampling.LANCZOS)


def main():
    rows = list(leaders())
    missing = [id for id, _, _ in rows if not (CLEAN / f"leader{id}.png").exists()]
    if missing:
        sys.exit(f"no clean token for {', '.join(missing)} in {CLEAN}")

    for id, value, colour in rows:
        clean = Image.open(CLEAN / f"leader{id}.png").convert("RGBA")
        size = max(clean.size)
        token = clean.resize((round(TOKEN_SCALE * size),) * 2, Image.Resampling.LANCZOS)
        out = Image.new("RGBA", (size, size))
        offset = (size - token.width) // 2
        out.paste(token, (offset, offset), token)
        out.alpha_composite(badge(size, value, colour))
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
