#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Build the Linux release files locally, as the release workflow does, without GitHub
# Actions: inside an Ubuntu 22.04 container (glibc 2.35, so the AppImage runs on Ubuntu
# 22.04+, Debian 12+, Fedora 36+), from a clean export of the tag.
#
# Usage: packaging/linux/build-release-local.sh <version> [output dir]
#   e.g. packaging/linux/build-release-local.sh 0.13.0 /tmp/release
# Needs podman (or CONTAINER=docker) and the tag v<version>. Writes the AppImage, the CLI
# archive, SHA256SUMS.txt and tests.log to the output dir. Then, to make the draft:
#   gh release create v<version> <out>/chipload-*.AppImage <out>/chipload-cli-*.tar.gz \
#     <out>/SHA256SUMS.txt --draft --verify-tag --title "Chipload <version>" --notes-file notes.md
set -eu

VERSION=$1
OUT=$(mkdir -p "${2:-dist/local}" && cd "${2:-dist/local}" && pwd)
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CONTAINER=${CONTAINER:-podman}

SRC=$(mktemp -d)
trap 'rm -rf "$SRC"' EXIT
git -C "$ROOT" archive "v$VERSION" | tar -x -C "$SRC"

cat > "$SRC/.build-in-container.sh" <<EOF
#!/bin/sh
set -eu
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq curl ca-certificates build-essential pkg-config file \\
    libxkbcommon0 libxkbcommon-x11-0 libxcb-xkb1 >/dev/null
# The toolchain comes from rust-toolchain.toml.
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none >/dev/null
. "\$HOME/.cargo/env"
cd /src
if ! packaging/test-release.sh > /out/tests.log 2>&1; then
    grep -E "FAILED|panicked|minimal failing input" /out/tests.log
    # A failing property test saved its case: keep it, to commit with the fix.
    find crates -name '*.proptest-regressions' -newer Cargo.toml -exec cp --parents {} /out/ \\;
    echo "TESTS FAILED"; exit 1
fi
cargo build --release --locked -p chipload-gui -p chipload-cli
REQUIRE_BUNDLED_LIBS=1 packaging/linux/build-appimage.sh "$VERSION" dist
stage="chipload-cli-$VERSION-linux-x86_64"
mkdir -p "\$stage"
cp target/release/chipload LICENSE THIRD-PARTY-LICENSES.txt "\$stage/"
tar -czf "dist/\$stage.tar.gz" "\$stage"
cd dist && sha256sum * > SHA256SUMS.txt && cp * /out/ && cat SHA256SUMS.txt
EOF

"$CONTAINER" run --rm -v "$SRC:/src:Z" -v "$OUT:/out:Z" docker.io/library/ubuntu:22.04 \
    sh /src/.build-in-container.sh
echo "Built into $OUT"
