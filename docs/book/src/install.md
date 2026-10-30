# Installing

Download Chipload from the [releases page](https://github.com/chipload/chipload/releases).
For the app you need one file: the one named `chipload-X.Y.Z-…` for your system (below).

Chipload is two programs that read and write the same project files and make the same
G-code:
- the desktop app, in the files named `chipload-X.Y.Z-…`;
- the command line `chipload`, for scripts and automation, in the archives named
  `chipload-cli-X.Y.Z-…`. You don't need it to use the app.

Each release also has the user guide to read offline and `SHA256SUMS.txt` to check the files.

## Linux

Download `chipload-X.Y.Z-linux-x86_64.AppImage`, make it executable and run it:

```sh
chmod +x chipload-*.AppImage
./chipload-*.AppImage                # the app; add a .chipload or .dxf file to open it
```

- **ARM64** (Raspberry Pi 4 or 5 with a 64-bit OS, ARM boards and laptops): take
  `chipload-X.Y.Z-linux-aarch64.AppImage` instead. The app needs OpenGL 3, which a Pi 4 or 5 has
  with its standard graphics driver. Not yet tried on a Pi: reports welcome.
- **Requirements:** glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 or newer,
  Raspberry Pi OS 12 "Bookworm").
- **Command line:** it's in `chipload-cli-X.Y.Z-linux-x86_64.tar.gz` (ARM64:
  `…-linux-aarch64.tar.gz`). Unpack it and put `chipload` on your `PATH`.
- **File dialogs:** these need an XDG desktop portal or `zenity`. Without them, use *File >
  Open path…* (and *Import image from path…*, *Update drawing from path…*), or give the files
  on the command line.

## Windows

Download `chipload-X.Y.Z-windows-x86_64-setup.exe` and run it. On a Windows laptop with an
ARM processor (Snapdragon), take `chipload-X.Y.Z-windows-aarch64-setup.exe` instead. It
installs Chipload for you alone, without asking for administrator rights, into
`%LOCALAPPDATA%\Programs\Chipload` (you can choose another folder), and then:

- `.chipload` files open in Chipload with a double-click, and show its icon;
- DXF and SVG files have Chipload under *Open with*; the program that opens them now
  stays as it is;
- *Chipload* is in the Start menu.

To update, run the newer installer: it replaces the old version in place. To remove
Chipload, use *Settings > Apps*. Your settings, tools, machines, materials and posts stay,
for when you install it again.

The installer isn't code-signed, so Windows SmartScreen may warn the first time. Choose
*More info > Run anyway*.

- **Without installing:** `chipload-X.Y.Z-windows-x86_64.exe` (`…-aarch64.exe`) is the app
  as one file, to run from anywhere, a USB stick too. Double-clicking a project won't open
  it; use *File > Open* in the app.
- **Command line:** the installer puts `chipload.exe` beside the app, in the folder above;
  run it from a terminal (PowerShell or Command Prompt) with its full path, e.g.
  `& "$env:LOCALAPPDATA\Programs\Chipload\chipload.exe" --version`. It comes only with
  the installer.

## macOS

No release has macOS files yet. Until one does, build Chipload from the source (below).
Once there: download `chipload-X.Y.Z-macos-universal.dmg`, open it and drag *Chipload* onto
*Applications*. It runs on Apple silicon and Intel Macs with macOS 11 or newer.

The app isn't notarised by Apple, so macOS refuses it the first time. Open *System Settings
> Privacy & Security*, scroll down to the message about Chipload and choose *Open Anyway*.

- **Command line:** it's in `chipload-cli-X.Y.Z-macos-universal.tar.gz`. Unpack it and put
  `chipload` on your `PATH`. If macOS stops it, run `xattr -d com.apple.quarantine chipload`
  once.
- **Opening files:** use *File > Open* in the app; double-clicking a `.chipload` file in
  Finder doesn't open it yet.

## From source

Install Rust with [rustup](https://rustup.rs), then:

```sh
git clone https://github.com/chipload/chipload
cd chipload
cargo build --release -p chipload-gui -p chipload-cli
# target/release/chipload-gui and target/release/chipload
```

## New versions

The first time it starts, Chipload asks whether to check for new versions. If you say yes,
it asks GitHub once a day, when it starts, for the latest release. That is one request:
nothing about you, your computer or your projects is sent, though GitHub sees your network
address as it does for any visit. When a newer version is out, the status bar says
*Chipload X is available*; click it for **What's new…** (the release's page in your
browser) or **Skip this version**. The same is under **Help**. Chipload never downloads or
installs anything by itself: download the new version as you did this one.

**Help > Check for new versions now** checks at once, also with the daily check off, and
says what it found. Change your answer in *This computer > Preferences > New versions*.
Behind a proxy, set `HTTPS_PROXY`.

## Your settings

Chipload keeps your tool library, machine profiles, own post-processors and app settings in
one folder:

| System | Folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/chipload/`, usually `~/.config/chipload/` |
| Windows | `%APPDATA%\chipload\` |
| macOS | `~/Library/Application Support/chipload/` |
| Any | the folder in the environment variable `CHIPLOAD_CONFIG_DIR`, if set |

A project keeps its own copy of the tools, machine and post it uses, so it cuts the same on
any computer, whatever that computer's library holds.
