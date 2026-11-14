#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The README's V-carve showcase: "Workshop" V-carved with a 25° V-bit
# (packaging/docs/showcase-text.chipload), its simulation played back as a GIF. Writes
# docs/book/src/images/showcase-text.gif. Needs Pillow (a build tool, not shipped).
#
# Usage: packaging/docs/text-showcase.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
out=$(realpath -m "${SHOTS_OUT:-docs/book/src/images}")
frames=$(mktemp -d)
trap 'rm -rf "$frames"' EXIT

CHIPLOAD_SHOTS_OUT="$frames" \
    cargo test -q --release -p chipload-gui --bin chipload-gui -- --ignored --exact \
    app::tests::screenshots::text_showcase --nocapture
python3 packaging/docs/frames-to-gif.py "$frames/text-frames" "$out/showcase-text.gif"
echo "Wrote $out/showcase-text.gif"
