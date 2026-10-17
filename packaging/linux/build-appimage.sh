#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Build chipload-gui-<version>-linux-<arch>.AppImage from target/release/chipload-gui, for the machine's
# architecture: x86_64 or aarch64 (ARM64, e.g. a Raspberry Pi 4 or 5).
#
# Usage: packaging/linux/build-appimage.sh <version> [output dir]
#
# Environment:
#   APPIMAGETOOL            path to appimagetool; otherwise the pinned release is downloaded
#                           and its sha256 checked.
#   REQUIRE_BUNDLED_LIBS=1  fail if a library to bundle is missing (set in CI).
set -eu

VERSION=$1
OUT=${2:-dist}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BINARY="$ROOT/target/release/chipload-gui"

ARCH=$(uname -m)
# appimagetool 1.9.1 (2025-11-18). The project publishes no checksums; these are the sha256 of
# the files as first downloaded, so a later change to them is caught.
case "$ARCH" in
    x86_64)
        APPIMAGETOOL_SHA256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
        LDCONFIG_ARCH=x86-64 ;;
    aarch64)
        APPIMAGETOOL_SHA256=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
        LDCONFIG_ARCH=AArch64 ;;
    *) echo "error: no AppImage for $ARCH (x86_64 and aarch64 only)" >&2; exit 1 ;;
esac
APPIMAGETOOL_URL=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$ARCH.AppImage

# Loaded at run time by winit for X11 keyboards; absent on minimal systems (e.g. WSL).
# MIT-licensed; listed in THIRD-PARTY-LICENSES.txt.
BUNDLED_LIBS="libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1"

[ -x "$BINARY" ] || { echo "error: build first: cargo build --release -p chipload-gui" >&2; exit 1; }

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
APPDIR="$WORK/Chipload.AppDir"

install -Dm755 "$BINARY" "$APPDIR/usr/bin/chipload-gui"
install -Dm755 "$ROOT/packaging/linux/AppRun" "$APPDIR/AppRun"
install -Dm644 "$ROOT/packaging/linux/chipload.desktop" "$APPDIR/chipload.desktop"
install -Dm644 "$ROOT/packaging/linux/chipload.desktop" "$APPDIR/usr/share/applications/chipload.desktop"
install -Dm644 "$ROOT/packaging/linux/chipload.svg" "$APPDIR/chipload.svg"
install -Dm644 "$ROOT/packaging/linux/chipload.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/chipload.svg"
install -Dm644 "$ROOT/LICENSE" "$APPDIR/usr/share/doc/chipload/LICENSE"
install -Dm644 "$ROOT/THIRD-PARTY-LICENSES.txt" "$APPDIR/usr/share/doc/chipload/THIRD-PARTY-LICENSES.txt"

for lib in $BUNDLED_LIBS; do
    path=$(/sbin/ldconfig -p | awk -v lib="$lib" -v arch="$LDCONFIG_ARCH" '$1 == lib && index($0, arch) { print $NF; exit }')
    if [ -n "$path" ]; then
        install -Dm644 "$path" "$APPDIR/usr/lib/$lib"
        # Its licence: the Debian/Ubuntu package's copyright file.
        package=$(dpkg -S "$(readlink -f "$path")" 2>/dev/null | head -n1 | cut -d: -f1)
        if [ -n "$package" ] && [ -f "/usr/share/doc/$package/copyright" ]; then
            install -Dm644 "/usr/share/doc/$package/copyright" \
                "$APPDIR/usr/share/doc/chipload/bundled/$package.copyright"
        elif [ "${REQUIRE_BUNDLED_LIBS:-0}" = 1 ]; then
            echo "error: no copyright file for $lib" >&2
            exit 1
        fi
    elif [ "${REQUIRE_BUNDLED_LIBS:-0}" = 1 ]; then
        echo "error: $lib not found to bundle" >&2
        exit 1
    else
        echo "warning: $lib not found; the AppImage will use the host's" >&2
    fi
done

if [ -z "${APPIMAGETOOL:-}" ]; then
    APPIMAGETOOL="$WORK/appimagetool"
    curl -sSL --fail -o "$APPIMAGETOOL" "$APPIMAGETOOL_URL"
    echo "$APPIMAGETOOL_SHA256  $APPIMAGETOOL" | sha256sum -c -
    chmod +x "$APPIMAGETOOL"
fi

mkdir -p "$OUT"
TARGET="$OUT/chipload-gui-$VERSION-linux-$ARCH.AppImage"
# Extract-and-run: CI machines and containers often have no FUSE.
ARCH=$ARCH APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" --no-appstream "$APPDIR" "$TARGET"
echo "Built $TARGET"
