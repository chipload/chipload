# 0010: Release builds: AppImage and single Windows executables via GitHub Actions

Status: accepted (2026-10-09)

## Decisions (2026-10-09)
- **Linux:** the GUI ships as an AppImage. **Windows:** the GUI ships as one `.exe`, no
  installer. The command line ships as a separate small asset per platform.
- A pushed `vX.Y.Z` tag builds everything and creates a **draft** GitHub Release; publishing
  is a manual step. Steps: `docs/releasing.md`.

## How
- `.github/workflows/release.yml` builds natively on `ubuntu-22.04` and `windows-latest`;
  `ci.yml` runs lint, tests, cargo-deny and the notices check on every push, and the tests
  on Windows. Only GitHub's own actions are used; tools are `cargo install --locked` at
  pinned versions. `rust-toolchain.toml` pins the compiler (1.99.0).
- **AppImage** (`packaging/linux/build-appimage.sh`): the GUI binary, desktop entry, icon,
  licences, and the X11 keyboard libraries winit loads at run time (`libxkbcommon`,
  `libxkbcommon-x11`, `libxcb-xkb`; MIT-style, their copyright files included), which minimal
  systems such as WSL lack. Wayland, X11 and OpenGL come from the host (loaded at run time);
  the binary itself links only libc, libm and libgcc_s. Built on Ubuntu 22.04, so it needs
  glibc 2.35 or newer. `appimagetool` 1.9.1 is pinned by sha256 (the project publishes no
  checksums; the pin catches later changes).
- **Windows:** `.cargo/config.toml` links the C runtime statically (`+crt-static`), so the
  `.exe` needs no Visual C++ redistributable; CI checks it has no VC runtime dependency. The
  GUI is a windowed program (no console); startup errors also show in a message box.
- **Licence notices:** `cargo-about` (MIT OR Apache-2.0, build tool only) generates
  `THIRD-PARTY-LICENSES.txt` for both targets from `about.toml` (same accepted list as
  `deny.toml`); CI fails if it's stale. `LICENSE` and the notices are compiled into every
  binary (GUI: Help > About and licences; CLI: `chipload licenses`) and are also in the
  AppImage and CLI archive.

## Not yet
- Code signing (Windows SmartScreen warns about unsigned executables; Azure Trusted Signing
  would be the cheapest option), an MSI installer, macOS builds, Linux arm64.
- Publishing requires the repository and tags to be pushed; whether the repository is
  public is a separate decision.
