#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The user guide's screenshots (docs/specs/user-guide.md §3, decision 3), made before each
# release: the GUI's test `screenshots` opens corpus projects headless (egui_kittest, wgpu),
# sets up each view and saves a frame. No display needed. Writes docs/book/src/images/*.png.
set -euo pipefail
cd "$(dirname "$0")/../.."

out=$(realpath -m "${SHOTS_OUT:-docs/book/src/images}")
mkdir -p "$out"

# The library picture: a settings folder of its own with two tools and a machine.
cargo build -q --release -p chipload-cli
library=$(mktemp -d)
trap 'rm -rf "$library"' EXIT
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload tool add "6 mm 2F upcut" \
    --diameter 6 --flutes 2 --rpm 18000 > /dev/null
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload tool add "V 60" --type vbit \
    --diameter 12.7 --angle 60 --tip 0.2 --rpm 18000 > /dev/null
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload machine add "Shapeoko" \
    --post grbl --safe-z 10 --travel 838,838,95 > /dev/null

# The guide is English. SHOTS_LANGUAGE=de (fr, es, it, sv) shows the app in another
# language, to check that its texts fit; SHOTS_OUT=DIR writes the pictures there instead
# of into the guide.
CHIPLOAD_SHOTS_OUT="$out" CHIPLOAD_SHOTS_LIBRARY="$library" \
    CHIPLOAD_SHOTS_LANGUAGE="${SHOTS_LANGUAGE:-en}" \
    cargo test -q --release -p chipload-gui --bin chipload-gui -- --ignored --exact \
    app::tests::screenshots::screenshots --nocapture
echo "Wrote $out/"
