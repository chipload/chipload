#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# The logo's PNGs and the Windows icon, from the SVGs make-brand.py writes
# (packaging/brand/README.md). Needs resvg (MPL-2.0): cargo install resvg --locked.
set -euo pipefail
cd "$(dirname "$0")"
RESVG=${RESVG:-resvg}
command -v "$RESVG" > /dev/null || { echo "needs resvg: cargo install resvg --locked" >&2; exit 1; }

python3 make-brand.py
mkdir -p png
for size in 16 24 32 48 64 128 256 512 1024; do
    "$RESVG" -w "$size" -h "$size" chipload-mark.svg "png/chipload-$size.png"
done
"$RESVG" -w 512 -h 512 chipload-avatar.svg png/chipload-avatar-512.png
"$RESVG" -w 1200 chipload-wordmark.svg png/chipload-wordmark.png

# The Windows icon: PNG images in an ICO container, one per size Windows asks for.
python3 - <<'PY'
import struct
sizes = [16, 24, 32, 48, 64, 128, 256]
images = [open(f"png/chipload-{s}.png", "rb").read() for s in sizes]
header = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
entries = b""
for size, data in zip(sizes, images):
    side = 0 if size == 256 else size          # 0 means 256 in an ICO entry
    entries += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
with open("chipload.ico", "wb") as f:
    f.write(header + entries + b"".join(images))
PY

# Where the app and the packages take them from.
cp chipload-mark.svg ../linux/chipload.svg
cp png/chipload-256.png ../../crates/chipload-gui/assets/chipload-256.png
cp chipload.ico ../../crates/chipload-gui/assets/chipload.ico
cp chipload-wordmark.svg ../../docs/book/src/images/chipload-wordmark.svg
cp chipload-mark.svg ../../docs/book/theme/favicon.svg
cp png/chipload-32.png ../../docs/book/theme/favicon.png
echo "Wrote the logo's files."
