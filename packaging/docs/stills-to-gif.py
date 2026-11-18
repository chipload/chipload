#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# A GIF from stills (the README's nesting showcase): each still held, a short cross-fade to
# the next, one palette for all, the last held longest. Needs Pillow (a build tool, not
# shipped).
#
# Usage: stills-to-gif.py STILLS_DIR OUT.gif
import glob
import os
import sys

from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
paths = sorted(glob.glob(os.path.join(src, "*.png")))
stills = [Image.open(p).convert("RGB") for p in paths]
HOLD_MS, LAST_MS, FADE_STEPS, FADE_MS = 2600, 4200, 8, 60

frames, durations = [], []
for k, still in enumerate(stills):
    frames.append(still)
    durations.append(LAST_MS if k == len(stills) - 1 else HOLD_MS)
    if k + 1 < len(stills):
        for step in range(1, FADE_STEPS + 1):
            frames.append(Image.blend(still, stills[k + 1], step / (FADE_STEPS + 1)))
            durations.append(FADE_MS)

# One palette for every frame, from all the stills side by side: small and steady.
sheet = Image.new("RGB", (stills[0].width * len(stills), stills[0].height))
for k, still in enumerate(stills):
    sheet.paste(still, (k * still.width, 0))
palette = sheet.quantize(colors=128, method=Image.Quantize.MEDIANCUT)
frames = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
frames[0].save(dst, save_all=True, append_images=frames[1:], duration=durations, loop=0,
               optimize=True)
print(f"{dst}: {len(frames)} frames, {os.path.getsize(dst) // 1024} KiB")
