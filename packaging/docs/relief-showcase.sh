#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The README's relief showcase: a 3DBenchy relief (the model by Creative Tools, CC0,
# https://www.printables.com/model/3161-3d-benchy) simulated in 3D, and its playback as a
# GIF. The project embeds the model, 15 MB, so it is kept out of the repository: pass its
# path. Writes docs/book/src/images/showcase-relief.png and showcase-relief.gif. Needs
# Pillow (a build tool, not shipped) for the GIF.
#
# Usage: packaging/docs/relief-showcase.sh path/to/benchy_relief.chipload
set -euo pipefail
cd "$(dirname "$0")/../.."
project=$(realpath "${1:?the relief project}")
out=$(realpath -m "${SHOTS_OUT:-docs/book/src/images}")
frames=$(mktemp -d)
trap 'rm -rf "$frames"' EXIT

CHIPLOAD_SHOTS_OUT="$frames" CHIPLOAD_RELIEF_SHOWCASE="$project" \
    cargo test -q --release -p chipload-gui --bin chipload-gui -- --ignored --exact \
    app::tests::screenshots::relief_showcase --nocapture
cp "$frames/showcase-relief.png" "$out/"
python3 - "$frames/relief-frames" "$out/showcase-relief.gif" <<'PY'
import glob, os, sys
from PIL import Image

src, dst = sys.argv[1], sys.argv[2]
paths = sorted(glob.glob(os.path.join(src, "*.png")))
# One palette for all frames (taken from the finished relief): small and steady.
# The canvas only, from under the ribbon to above the navigation bar (a 960 × 600 window).
frames = [Image.open(p).convert("RGB").crop((0, 102, 960, 468)) for p in paths]
palette = frames[-1].quantize(colors=128, method=Image.Quantize.MEDIANCUT)
frames = [f.quantize(palette=palette, dither=Image.Dither.NONE) for f in frames]
# The last frame held, so the finished relief stays a moment before it starts again.
durations = [90] * (len(frames) - 1) + [2500]
frames[0].save(dst, save_all=True, append_images=frames[1:], duration=durations, loop=0,
               optimize=True)
print(f"{dst}: {len(frames)} frames, {os.path.getsize(dst) // 1024} KiB")
PY
echo "Wrote $out/showcase-relief.png and showcase-relief.gif"
