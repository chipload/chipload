#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The user guide's screenshots (docs/specs/user-guide.md §3, decision 3), made before each
# release: the app opens a corpus project, sets up a view, saves its first frame and exits.
# Needs a display (X11 or Wayland). Writes docs/book/src/images/*.png.
set -euo pipefail
cd "$(dirname "$0")/../.."

cargo build -q --release -p chipload-gui --features screenshots
out=docs/book/src/images
mkdir -p "$out"
# An empty settings folder: the pictures don't depend on this computer's library.
settings=$(mktemp -d)
trap 'rm -rf "$settings"' EXIT

shot() { # name project steps
    echo "$1.png"
    CHIPLOAD_CONFIG_DIR="$settings" CHIPLOAD_SCREENSHOT_SETUP="$3" \
        EFRAME_SCREENSHOT_TO="$out/$1.png" target/release/chipload-gui "$2" > /dev/null 2>&1
}

shot toolpath tests/corpus/jobs/v18/dogbones.chipload generate
shot simulation-3d tests/corpus/jobs/v18/dogbones.chipload simulation,3d
shot playback tests/corpus/jobs/v18/dogbones.chipload simulation,half
shot image tests/corpus/jobs/v19/image_vcarve.chipload image
shot drawing tests/corpus/jobs/v21/two_dxfs.chipload drawing
shot properties tests/corpus/jobs/v18/dogbones.chipload generate,select
shot parts tests/corpus/jobs/v22/two_parts.chipload generate,part
echo "Wrote $out/"
