#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Build chipload-<version>-windows-<arch>-setup.exe, the Windows installer
# (docs/specs/windows-installer.md), from a Windows build of the app and the command line.
# Runs on Linux: NSIS's makensis builds Windows installers on any system (Ubuntu: `nsis`).
#
# Usage: packaging/windows/build-installer.sh <version> <x86_64|aarch64> <chipload-gui.exe>
#            <chipload.exe> [output dir]
#
# Environment:
#   MAKENSIS  path to makensis (default: makensis on PATH); NSISDIR as makensis needs it.
set -eu

VERSION=$1
ARCH=$2
GUI=$3
CLI=$4
OUT=${5:-dist}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
MAKENSIS=${MAKENSIS:-makensis}

case "$ARCH" in
    x86_64 | aarch64) ;;
    *) echo "error: no installer for $ARCH (x86_64 and aarch64 only)" >&2; exit 1 ;;
esac
for file in "$GUI" "$CLI"; do
    [ -f "$file" ] || { echo "error: no $file" >&2; exit 1; }
done

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
cp "$GUI" "$STAGE/chipload-gui.exe"
cp "$CLI" "$STAGE/chipload.exe"
cp "$ROOT/LICENSE" "$ROOT/THIRD-PARTY-LICENSES.txt" "$STAGE/"
cp "$ROOT/crates/chipload-gui/assets/chipload.ico" "$STAGE/"

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
"$MAKENSIS" -V2 -WX -DVERSION="$VERSION" -DARCH="$ARCH" -DSTAGE="$STAGE" -DOUTDIR="$OUT" \
    "$ROOT/packaging/windows/chipload.nsi"
echo "$OUT/chipload-$VERSION-windows-$ARCH-setup.exe"
