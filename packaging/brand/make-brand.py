#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Chipload's logo, drawn from geometry (packaging/brand/README.md).

The mark is a C made of a pocket's rings, spiralling in, with the tool (an orange dot)
where the cut starts. The wordmark puts that C, half again as tall, before HIPLOAD in
rounded strokes. Everything is plain SVG paths: no font is needed to show it.

    python3 packaging/brand/make-brand.py      # writes the SVGs next to this script
    packaging/brand/render.sh                  # then the PNGs and the Windows icon
"""
import math
import os

GREEN = "#9ab964"   # the brand colour
ORANGE = "#f29e38"  # the tool: the one accent
INK = "#f3f5f0"     # rings on the green tile
DARK = "#1d2025"    # text and backgrounds in dark places

HERE = os.path.dirname(os.path.abspath(__file__))


def pt(cx, cy, r, degrees):
    """A point on a circle, angles counter-clockwise from +X, in SVG's Y-down space."""
    a = math.radians(degrees)
    return (cx + r * math.cos(a), cy - r * math.sin(a))


def fmt(p):
    return f"{p[0]:.2f},{p[1]:.2f}"


def arc_to(cx, cy, r, a0, a1, ccw):
    large = 1 if abs(a1 - a0) > 180 else 0
    return f"A{r:.2f},{r:.2f} 0 {large} {0 if ccw else 1} {fmt(pt(cx, cy, r, a1))}"


def stroked(d, color, width, transform=""):
    t = f' transform="{transform}"' if transform else ""
    return (f'<path d="{d}"{t} fill="none" stroke="{color}" stroke-width="{width:.2f}" '
            f'stroke-linecap="round" stroke-linejoin="round"/>')


def rings(cx, cy, radii, width, color, tool_r, a0=42, a1=318):
    """The C: rings from the outside in, joined at the opening as a pocket links them,
    and the tool at the start of the outer ring."""
    d = ""
    for i, r in enumerate(radii):
        start, end = (a0, a1) if i % 2 == 0 else (a1, a0)
        d += ("M" if i == 0 else " L") + fmt(pt(cx, cy, r, start)) + " "
        d += arc_to(cx, cy, r, start, end, i % 2 == 0)
    tool = pt(cx, cy, radii[0], a0)
    return (stroked(d, color, width)
            + f'<circle cx="{tool[0]:.2f}" cy="{tool[1]:.2f}" r="{tool_r:.2f}" fill="{ORANGE}"/>')


# The letters: centre lines in a box 100 high, drawn with round ends and joins.
W = 26
M = W / 2


def letter_h(x):
    return f"M{x + M},{M} V{100 - M} M{x + 77},{M} V{100 - M} M{x + M},50 H{x + 77}", 90


def letter_i(x):
    return f"M{x + M},{M} V{100 - M}", 26


def letter_p(x):
    return f"M{x + M},{100 - M} V{M} H{x + 46} A21.5,21.5 0 0 1 {x + 46},56 H{x + M}", 82


def letter_l(x):
    return f"M{x + M},{M} V{100 - M} H{x + 66}", 74


def letter_o(x):
    return f"M{x + 87},50 A37,37 0 1 0 {x + 13},50 A37,37 0 1 0 {x + 87},50", 100


def letter_a(x):
    r = (77 - M) / 2
    return (f"M{x + M},{100 - M} V50 A{r},{r} 0 0 1 {x + 77},50 V{100 - M} "
            f"M{x + M},64 H{x + 77}"), 90


def letter_d(x):
    return f"M{x + M},{M} V{100 - M} H{x + 40} A37,37 0 0 0 {x + 40},{M} Z", 90


GAP = 16


def hipload():
    d, x = "", 0.0
    for letter in (letter_h, letter_i, letter_p, letter_l, letter_o, letter_a, letter_d):
        path, advance = letter(x)
        d += path + " "
        x += advance + GAP
    return d, x - GAP


def svg(body, w, h, title):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.0f} {h:.0f}" '
            f'width="{w:.0f}" height="{h:.0f}" role="img" aria-label="{title}">'
            f'<title>{title}</title>{body}</svg>\n')


def mark_tile():
    """The app icon: white rings on a green tile."""
    tile = f'<rect x="8" y="8" width="240" height="240" rx="56" fill="{GREEN}"/>'
    return tile + rings(128, 128, (86, 60, 34), 17, INK, 19.5)


def avatar():
    """A square picture for the GitHub organisation, which rounds its corners itself."""
    return f'<rect width="256" height="256" fill="{GREEN}"/>' + rings(128, 128, (82, 57, 32), 16, INK, 18.5)


def mark_plain(color):
    """The C alone, for small places on a plain background (the app's top bar, favicons)."""
    return rings(128, 128, (100, 70, 40), 20, color, 23)


def wordmark(color):
    """The ring C, 1.5 times the letters' height, then HIPLOAD. Returns (svg body, w, h)."""
    sw, top = 17, 75
    r = top - sw / 2
    c = rings(top, 50, (r, r - sw - 9, r - 2 * (sw + 9)), sw, color, 19)
    letters, width = hipload()
    x = 2 * top + 2
    body = c + stroked(letters, color, 22, f"translate({x},0)")
    # Room for the C above and below the letters, and for the tool.
    pad = top - 50 + 4
    return f'<g transform="translate(4,{pad})">{body}</g>', x + width + 8, 100 + 2 * pad


def write(name, text):
    with open(os.path.join(HERE, name), "w", encoding="utf-8") as f:
        f.write(text)


def main():
    write("chipload-mark.svg", svg(mark_tile(), 256, 256, "Chipload"))
    write("chipload-avatar.svg", svg(avatar(), 256, 256, "Chipload"))
    write("chipload-mark-green.svg", svg(mark_plain(GREEN), 256, 256, "Chipload"))
    write("chipload-mark-white.svg", svg(mark_plain(INK), 256, 256, "Chipload"))
    for name, color in (("chipload-wordmark.svg", GREEN), ("chipload-wordmark-white.svg", INK)):
        body, w, h = wordmark(color)
        write(name, svg(body, w, h, "Chipload"))


if __name__ == "__main__":
    main()
