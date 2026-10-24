#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The user guide's screenshots (docs/specs/user-guide.md §3, decision 3), made before each
# release: the app opens a corpus project, sets up a view, saves its first frame and exits.
# Needs a display (X11 or Wayland). Writes docs/book/src/images/*.png.
set -euo pipefail
cd "$(dirname "$0")/../.."

cargo build -q --release -p chipload-gui --features screenshots
out=${SHOTS_OUT:-docs/book/src/images}
mkdir -p "$out"
# An empty settings folder: the pictures don't depend on this computer's library.
settings=$(mktemp -d)
trap 'rm -rf "$settings"' EXIT

# The guide is English, and the app follows the system's language
# (docs/specs/translations.md), which it reads from LANGUAGE first. SHOTS_LANGUAGE=de (fr,
# es, it, sv) shows the app in another language, to check that its texts fit; SHOTS_OUT=DIR
# writes the pictures there instead of into the guide.
export LANGUAGE="${SHOTS_LANGUAGE:-en}"

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
# The README's picture: a larger window, the simulated stock with the toolpaths over it,
# the logo V-carved from a traced picture (the picture itself hidden), the outline open.
CHIPLOAD_SCREENSHOT_SIZE=1600x950 shot showcase packaging/docs/showcase.chipload \
    simulation,3d,noreport,noimages,op4
# The Text page: a text and its properties.
shot text tests/corpus/jobs/v27/text.chipload generate,select,text1
# The library: a settings folder of its own with two tools and a machine.
cargo build -q --release -p chipload-cli
library=$(mktemp -d)
trap 'rm -rf "$settings" "$library"' EXIT
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload tool add "6 mm 2F upcut" \
    --diameter 6 --flutes 2 --rpm 18000 > /dev/null
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload tool add "V 60" --type vbit \
    --diameter 12.7 --angle 60 --tip 0.2 --rpm 18000 > /dev/null
CHIPLOAD_CONFIG_DIR="$library" target/release/chipload machine add "Shapeoko" \
    --post grbl --safe-z 10 --travel 838,838,95 > /dev/null
echo "library.png"
CHIPLOAD_CONFIG_DIR="$library" CHIPLOAD_SCREENSHOT_SETUP=library \
    EFRAME_SCREENSHOT_TO="$out/library.png" target/release/chipload-gui \
    tests/corpus/jobs/v22/two_parts.chipload > /dev/null 2>&1
echo "Wrote $out/"
