# 0010: Release builds: AppImage and single Windows executables via GitHub Actions

Status: accepted (2026-10-09)

## Decisions (2026-10-09)
- **Linux:** the GUI ships as an AppImage. **Windows:** the GUI ships as one `.exe`, no
  installer. The command line ships as a separate small asset per platform.
- A pushed `vX.Y.Z` tag builds everything and creates a **draft** GitHub Release; publishing
  is a manual step. Steps: `docs/releasing.md`.

## Decisions (2026-10-24): file names
- The app takes the plain name, `chipload-X.Y.Z-<system>-<arch>.{AppImage,exe,dmg}`; the
  command line is `chipload-cli-X.Y.Z-…`, always in an archive (`.zip` on Windows, `.tar.gz`
  elsewhere) with LICENSE and the notices. Before, the command line was `chipload-X.Y.Z-…`
  (a bare `.exe` on Windows) and the app `chipload-gui-X.Y.Z-…`: the command line sorted
  first on the release page, and double-clicking it showed a console window that closed at
  once, which looks like an app that won't start. The programs inside keep their names
  (`chipload`, `chipload-gui`). The release notes start with the app downloads.

## Decisions (2026-10-16): macOS
- **macOS:** the GUI ships as `Chipload.app` in a disk image, the command line as a
  `.tar.gz`, both **universal** (Apple silicon and Intel), for macOS 11 or newer. Built by
  the release workflow on `macos-15` (Apple silicon; the Intel half cross-compiled) and
  joined with `lipo` (`packaging/macos/build-macos.sh`). Signed **ad hoc** only: no Apple
  Developer ID and no notarisation yet, so Gatekeeper refuses the first start until the
  user chooses *Open Anyway*. The build fails if a binary links anything outside the
  system's own libraries. Bundle identifier `se.gntech.chipload`.
- The notices cover the macOS targets too (`about.toml`).
- Not built locally: there is no Mac here. The first run of the job will be its first test.

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
  would be the cheapest option), an MSI installer, Linux arm64. On macOS: Developer ID
  signing and notarisation (a paid Apple Developer account; the certificate and password would come from repository
  secrets), opening a `.chipload` from Finder (the app takes files only as arguments).
- Publishing requires the repository and tags to be pushed; whether the repository is
  public is a separate decision.
