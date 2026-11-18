#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The README's nesting showcase: the nesting demo (tests/corpus/jobs/v41/nesting_demo.chipload)
# simulated as drawn, the Nest window, and the boards nested, cross-faded into a GIF. Writes
# docs/book/src/images/showcase-nest.gif. Needs Pillow (a build tool, not shipped).
#
# Usage: packaging/docs/nest-showcase.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
out=$(realpath -m "${SHOTS_OUT:-docs/book/src/images}")
frames=$(mktemp -d)
trap 'rm -rf "$frames"' EXIT

CHIPLOAD_SHOTS_OUT="$frames" \
    cargo test -q --release -p chipload-gui --bin chipload-gui -- --ignored --exact \
    app::tests::screenshots::nest_showcase --nocapture
python3 packaging/docs/stills-to-gif.py "$frames/nest-frames" "$out/showcase-nest.gif"
echo "Wrote $out/showcase-nest.gif"
