# /// script
# dependencies = ["pillow==11.3.0"]
# ///
"""Draws each leader's strength in a badge on its token.

The original token art (the person*.png and .gif files of the C# client) has the
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

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parent.parent
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


# Each leader's strength by id, as the engine's leader table has it
# (rust/engine/src/heroes). Zoal (1036) has none and shows an X.
STRENGTH = {
    1001: "5", 1002: "5", 1003: "4", 1004: "2", 1005: "1", 1006: "5", 1007: "5", 1008: "5",
    1009: "5", 1010: "5", 1011: "6", 1012: "5", 1013: "3", 1014: "3", 1015: "2", 1016: "7",
    1017: "6", 1018: "5", 1019: "3", 1020: "2", 1021: "5", 1022: "3", 1023: "3", 1024: "2",
    1025: "1", 1026: "6", 1027: "4", 1028: "3", 1029: "2", 1030: "1", 1031: "4", 1032: "5",
    1033: "5", 1034: "2", 1035: "1", 1036: "X", 1037: "4", 1038: "3", 1039: "2", 1040: "1",
    1041: "2", 1042: "3", 1043: "3", 1044: "4", 1045: "4", 1046: "2", 1047: "2", 1048: "2",
    1049: "3", 1050: "4", 1051: "5", 1052: "4", 1053: "3", 1054: "3", 1055: "2", 1056: "4",
    1057: "6", 1058: "5", 1059: "4", 1060: "2", 1061: "4", 1062: "1",
}


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
    rows = list(STRENGTH.items())
    missing = [id for id, _ in rows if not (CLEAN / f"leader{id}.png").exists()]
    if missing:
        sys.exit(f"no clean token for {', '.join(missing)} in {CLEAN}")

    for id, value in rows:
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
        tiles = [Image.open(ART / f"leader{id}.png").resize((150, 150)) for id, _ in rows]
        sheet = Image.new("RGBA", (150 * 12, 150 * ((len(tiles) + 11) // 12)), (40, 20, 5, 255))
        for i, t in enumerate(tiles):
            sheet.paste(t, ((i % 12) * 150, (i // 12) * 150), t)
        sheet.save("/tmp/leader-tokens.png")


if __name__ == "__main__":
    main()
