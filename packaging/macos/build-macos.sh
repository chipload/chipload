#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the macOS release files from the two per-architecture release builds:
#   Chipload-<version>-macos-universal.dmg        the GUI as Chipload.app (Apple silicon + Intel)
#   chipload-<version>-macos-universal.tar.gz     the command line, LICENSE, notices
#
# Usage: packaging/macos/build-macos.sh <version> [output dir]
#
# Needs (on macOS): target/{aarch64,x86_64}-apple-darwin/release/{chipload-gui,chipload}
# built first, Xcode's command line tools (lipo, iconutil, codesign, hdiutil, otool) and
# rsvg-convert (Homebrew librsvg) for the icon.
#
# The app is signed ad hoc only (no Developer ID, no notarisation; ADR 0010): Gatekeeper asks
# before the first start.
set -eu

VERSION=$1
OUT=${2:-dist}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
ARM="$ROOT/target/aarch64-apple-darwin/release"
INTEL="$ROOT/target/x86_64-apple-darwin/release"
# The oldest macOS the app starts on; the build sets MACOSX_DEPLOYMENT_TARGET to the same.
MIN_MACOS=11.0

for binary in "$ARM/chipload-gui" "$ARM/chipload" "$INTEL/chipload-gui" "$INTEL/chipload"; do
    [ -x "$binary" ] || {
        echo "error: build first: cargo build --release --target aarch64-apple-darwin" \
            "(and x86_64-apple-darwin) -p chipload-gui -p chipload-cli" >&2
        exit 1
    }
done

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# One binary for both architectures.
lipo -create "$ARM/chipload-gui" "$INTEL/chipload-gui" -output "$WORK/chipload-gui"
lipo -create "$ARM/chipload" "$INTEL/chipload" -output "$WORK/chipload"

# Only the system's own libraries: the app must run on a Mac without Homebrew.
for binary in "$WORK/chipload-gui" "$WORK/chipload"; do
    foreign=$(otool -L "$binary" | tail -n +2 | awk '{ print $1 }' \
        | grep -v -e '^/usr/lib/' -e '^/System/Library/' || true)
    if [ -n "$foreign" ]; then
        echo "error: $(basename "$binary") links libraries outside the system:" >&2
        echo "$foreign" >&2
        exit 1
    fi
done

# The app bundle.
APP="$WORK/dmg/Chipload.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$WORK/chipload-gui" "$APP/Contents/MacOS/chipload-gui"
cp "$ROOT/LICENSE" "$ROOT/THIRD-PARTY-LICENSES.txt" "$APP/Contents/Resources/"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@MIN_MACOS@/$MIN_MACOS/g" \
    "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"

# The icon, from the same SVG as Linux's.
ICONSET="$WORK/chipload.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
    rsvg-convert -w "$size" -h "$size" "$ROOT/packaging/linux/chipload.svg" \
        -o "$ICONSET/icon_${size}x${size}.png"
    double=$((size * 2))
    rsvg-convert -w "$double" -h "$double" "$ROOT/packaging/linux/chipload.svg" \
        -o "$ICONSET/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/chipload.icns"

# Ad hoc: Apple silicon refuses to run unsigned code, and lipo and the bundle need a fresh
# signature over the whole app.
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

mkdir -p "$OUT"

# The disk image: the app and a link to Applications to drag it onto.
ln -s /Applications "$WORK/dmg/Applications"
DMG="$OUT/Chipload-$VERSION-macos-universal.dmg"
rm -f "$DMG"
hdiutil create -volname "Chipload $VERSION" -srcfolder "$WORK/dmg" -fs HFS+ -format UDZO \
    -ov "$DMG"
echo "Built $DMG"

# The command line, signed ad hoc as well.
STAGE="$WORK/chipload-$VERSION-macos-universal"
mkdir -p "$STAGE"
cp "$WORK/chipload" "$ROOT/LICENSE" "$ROOT/THIRD-PARTY-LICENSES.txt" "$STAGE/"
codesign --force --sign - "$STAGE/chipload"
tar -czf "$OUT/chipload-$VERSION-macos-universal.tar.gz" -C "$WORK" "$(basename "$STAGE")"
echo "Built $OUT/chipload-$VERSION-macos-universal.tar.gz"
