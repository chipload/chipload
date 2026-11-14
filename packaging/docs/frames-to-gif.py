#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# The README's GIFs from a playback's frames (960 wide window renders, as the GUI's tests
# playback_showcase writes them): the canvas only, one palette taken from the finished
# stock, the last frame held. Needs Pillow (a build tool, not shipped).
#
# Usage: frames-to-gif.py FRAMES_DIR OUT.gif
import glob
import os
import sys

from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
paths = sorted(glob.glob(os.path.join(src, "*.png")))
# The canvas only, from under the ribbon (102 px) to above the navigation bar (132 px up
# from the window's bottom).
def canvas(path):
    image = Image.open(path).convert("RGB")
    return image.crop((0, 102, image.width, image.height - 132))


frames = [canvas(p) for p in paths]
# One palette for all frames: small and steady.
palette = frames[-1].quantize(colors=128, method=Image.Quantize.MEDIANCUT)
frames = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
# The finished stock stays a moment before it starts again.
durations = [90] * (len(frames) - 1) + [2500]
frames[0].save(dst, save_all=True, append_images=frames[1:], duration=durations, loop=0,
               optimize=True)
print(f"{dst}: {len(frames)} frames, {os.path.getsize(dst) // 1024} KiB")
