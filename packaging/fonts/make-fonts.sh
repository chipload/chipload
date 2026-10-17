#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Make the built-in fonts of chipload-text (docs/specs/text.md §2, more-fonts.md §2): GNU
# FreeFont 20120503 and ten Apache-2.0 fonts from Google Fonts, cut down to the Latin
# letters, digits and common symbols. The results are committed in
# crates/chipload-text/fonts/; run this only to make them again.
#
# Usage: packaging/fonts/make-fonts.sh
#
# Needs curl, python3 and network access. fontTools (MIT, a build tool only, not shipped) is
# installed in a temporary virtual environment.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT="$ROOT/crates/chipload-text/fonts"
URL=https://ftp.gnu.org/gnu/freefont/freefont-ttf-20120503.zip
# The file as released (its GPG signature by Steve White, FreeFont's maintainer, verified
# against the GNU keyring when this was pinned, 2026-10-16).
SHA256=7c85baf1bf82a1a1845d1322112bc6ca982221b484e3b3925022e25b5cae89af
FACES="FreeSans FreeSansBold FreeSerif FreeSerifBold FreeMono FreeMonoBold"
# Basic Latin, Latin-1, Latin Extended-A, dashes and quotes, bullet, ellipsis, per mille,
# euro, trademark, minus, diameter.
UNICODES="U+0020-007E,U+00A0-017F,U+2013-2014,U+2018-201E,U+2022,U+2026,U+2030,U+20AC,\
U+2122,U+2212,U+2300"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
curl -sSfL -o "$WORK/freefont.zip" "$URL"
echo "$SHA256  $WORK/freefont.zip" | sha256sum -c -
python3 -I -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' \
    "$WORK/freefont.zip" "$WORK"
python3 -m venv "$WORK/venv"
"$WORK/venv/bin/pip" install -q fonttools==4.54.1

mkdir -p "$OUT"
for face in $FACES; do
    # Kerning is kept, in the GPOS kern feature and the old kern table (chipload-text reads
    # both); hinting and the other layout features aren't used. The name table keeps the
    # copyright and licence notices.
    "$WORK/venv/bin/pyftsubset" "$WORK/freefont-20120503/$face.ttf" \
        --unicodes="$UNICODES" --legacy-kern --layout-features='kern' --no-hinting \
        --name-IDs='*' --name-languages='*' --output-file="$OUT/$face.ttf"
done
# The Apache-2.0 fonts (docs/specs/more-fonts.md, ADR 0021), from a fixed commit of the
# Google Fonts repository, each file checked by its SHA-256.
GF=https://raw.githubusercontent.com/google/fonts/47d54ca1ebb14fd864695f1d39dc734f39017c33/apache
APACHE="
satisfy/Satisfy-Regular.ttf 55bb141a0a23a32278d5fa7fd2e853824faa173b94774660ebeddb106be07c80 Satisfy
yellowtail/Yellowtail-Regular.ttf 3e06841b13000975f3cc38aa84e326e601a4378a4c52d50b5e9fee3598c5d236 Yellowtail
rochester/Rochester-Regular.ttf 38657df5e357e841a39995592970ac14ea96a1376f178f0d8e0231fd5d630b2b Rochester
luckiestguy/LuckiestGuy-Regular.ttf cfbdd68a039f92df51cf3721506af6242e64594c6325fe0bedbeff3fe385d980 LuckiestGuy
ultra/Ultra-Regular.ttf 9ee63cffbc379e5d4eb5fed6a0b94b770da919497e5cba4a227a6393dae0939e Ultra
chewy/Chewy-Regular.ttf 7cf75ea288f82fd20badea8ab4da7a656a96a7277c170811e813b3d3d6294147 Chewy
smokum/Smokum-Regular.ttf dfb06e95e7b99724e2e508979bdd679c2a1a9c8413c7f6961656097fd38c9a63 Smokum
cherrycreamsoda/CherryCreamSoda-Regular.ttf 158577b383ce9798384580bd41c35cd0fb4ed550a32fe4a2e41f4f2594337385 CherryCreamSoda
permanentmarker/PermanentMarker-Regular.ttf 28f82c8a7943cb8e9d599f8554da1d4fc75dbcf69b9885ad6c0611d20c6946c5 PermanentMarker
"
subset() { # in out
    "$WORK/venv/bin/pyftsubset" "$1" --unicodes="$UNICODES" --legacy-kern \
        --layout-features='kern' --no-hinting --name-IDs='*' --name-languages='*' \
        --output-file="$2"
}
echo "$APACHE" | while read -r path sum face; do
    [ -n "$path" ] || continue
    curl -sSfL -o "$WORK/$face.ttf" "$GF/$path"
    echo "$sum  $WORK/$face.ttf" | sha256sum -c -
    subset "$WORK/$face.ttf" "$OUT/$face.ttf"
done
# Roboto Slab is a variable font: two static faces, Regular (400) and Bold (700).
curl -sSfL -o "$WORK/RobotoSlabVF.ttf" "$GF/robotoslab/RobotoSlab%5Bwght%5D.ttf"
echo "786ae192477447d33c6672c3055fba7cbfe45184c9a79e77a14f15716ca05b16  $WORK/RobotoSlabVF.ttf" \
    | sha256sum -c -
for weight in 400:RobotoSlab 700:RobotoSlabBold; do
    "$WORK/venv/bin/fonttools" varLib.instancer "$WORK/RobotoSlabVF.ttf" "wght=${weight%%:*}" \
        -q -o "$WORK/${weight#*:}.ttf"
    subset "$WORK/${weight#*:}.ttf" "$OUT/${weight#*:}.ttf"
done
curl -sSfL -o "$WORK/LICENSE-Apache-2.0.txt" "$GF/satisfy/LICENSE.txt"
echo "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30  $WORK/LICENSE-Apache-2.0.txt" \
    | sha256sum -c -
cp "$WORK/LICENSE-Apache-2.0.txt" "$OUT/LICENSE-Apache-2.0.txt"
echo "Wrote $OUT"
ls -l "$OUT"
