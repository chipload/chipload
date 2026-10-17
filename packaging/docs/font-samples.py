#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""The user guide's picture of the built-in fonts (docs/book/src/images/fonts.png), drawn
from the font files in crates/chipload-text/fonts with Pillow (a build tool, not shipped).

Usage: python3 packaging/docs/font-samples.py
"""
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
FONTS = os.path.join(ROOT, "crates", "chipload-text", "fonts")
OUT = os.path.join(ROOT, "docs", "book", "src", "images", "fonts.png")
GROUPS = [
    ("Plain", [("FreeSans", "FreeSans"), ("FreeSans Bold", "FreeSansBold"),
               ("FreeSerif", "FreeSerif"), ("FreeSerif Bold", "FreeSerifBold"),
               ("FreeMono", "FreeMono"), ("FreeMono Bold", "FreeMonoBold"),
               ("Roboto Slab", "RobotoSlab"), ("Roboto Slab Bold", "RobotoSlabBold")]),
    ("Script", [("Satisfy", "Satisfy"), ("Yellowtail", "Yellowtail"),
                ("Rochester", "Rochester")]),
    ("Display", [("Luckiest Guy", "LuckiestGuy"), ("Ultra", "Ultra"), ("Chewy", "Chewy"),
                 ("Smokum", "Smokum"), ("Cherry Cream Soda", "CherryCreamSoda"),
                 ("Permanent Marker", "PermanentMarker")]),
]
SAMPLE = "Maker Space · Oak & Iron 1926"
ROW, LABEL_W, W = 64, 300, 1400

rows = sum(len(fonts) + 1 for _, fonts in GROUPS)
image = Image.new("RGB", (W, rows * ROW + 24), (255, 255, 255))
draw = ImageDraw.Draw(image)
label = ImageFont.truetype(os.path.join(FONTS, "FreeSans.ttf"), 22)
heading = ImageFont.truetype(os.path.join(FONTS, "FreeSansBold.ttf"), 22)
y = 12
for title, fonts in GROUPS:
    draw.text((24, y + 30), title.upper(), font=heading, fill=(111, 143, 60))
    y += ROW
    for name, file in fonts:
        face = ImageFont.truetype(os.path.join(FONTS, file + ".ttf"), 40)
        draw.text((24, y + 22), name, font=label, fill=(90, 90, 90))
        draw.text((LABEL_W, y + 8), SAMPLE, font=face, fill=(29, 32, 37))
        y += ROW
image.save(OUT, optimize=True)
print(OUT)
