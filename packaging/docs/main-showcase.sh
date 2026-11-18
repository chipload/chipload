#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The README's first picture as a GIF: the plaque (packaging/docs/showcase.chipload) played
# back in the simulation in 3D. Writes docs/book/src/images/showcase-sim.gif. Needs Pillow
# (a build tool, not shipped). The still showcase.png stays (the guide and the social
# preview); renew it with packaging/docs/screenshots.sh.
#
# Usage: packaging/docs/main-showcase.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
out=$(realpath -m "${SHOTS_OUT:-docs/book/src/images}")
frames=$(mktemp -d)
trap 'rm -rf "$frames"' EXIT

CHIPLOAD_SHOTS_OUT="$frames" \
    cargo test -q --release -p chipload-gui --bin chipload-gui -- --ignored --exact \
    app::tests::screenshots::main_showcase --nocapture
python3 packaging/docs/frames-to-gif.py "$frames/plaque-frames" "$out/showcase-sim.gif"
echo "Wrote $out/showcase-sim.gif"
