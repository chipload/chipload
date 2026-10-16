#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Make the built-in fonts of chipload-text (docs/specs/text.md §2): GNU FreeFont 20120503,
# cut down to the Latin letters, digits and common symbols. The results are committed in
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
echo "Wrote $OUT"
ls -l "$OUT"
