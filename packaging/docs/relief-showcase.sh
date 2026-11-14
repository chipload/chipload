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
python3 packaging/docs/frames-to-gif.py "$frames/relief-frames" "$out/showcase-relief.gif"
echo "Wrote $out/showcase-relief.png and showcase-relief.gif"
