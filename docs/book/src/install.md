# Installing

Chipload is two programs:
- `chipload-gui`, the desktop app;
- `chipload`, the command line.

Both read and write the same project files, and both make the same G-code. Download them
from the [releases page](https://github.com/gntech/chipload/releases): each release has the
files below, the user guide to read offline, and `SHA256SUMS.txt` to check them.

## Linux

Download `Chipload-X.Y.Z-x86_64.AppImage`, make it executable and run it:

```sh
chmod +x Chipload-*.AppImage
./Chipload-*.AppImage            # the app; add a .chipload or .dxf file to open it
```

- **Requirements:** glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 or newer).
- **Command line:** it's in `chipload-X.Y.Z-linux-x86_64.tar.gz`. Unpack it and put
  `chipload` on your `PATH`.
- **File dialogs:** these need an XDG desktop portal or `zenity`. Without them, use *File >
  Open path…* (and *Import image from path…*, *Update drawing from path…*), or give the files
  on the command line.

## Windows

Download `chipload-gui-X.Y.Z-windows-x86_64.exe`, the app as one file with nothing to
install, and `chipload-X.Y.Z-windows-x86_64.exe`, the command line.

The files aren't code-signed, so Windows SmartScreen may warn the first time. Choose *More
info > Run anyway*.

## macOS

No release has macOS files yet. Until one does, build Chipload from the source (below).
Once there: download `Chipload-X.Y.Z-macos-universal.dmg`, open it and drag *Chipload* onto
*Applications*. It runs on Apple silicon and Intel Macs with macOS 11 or newer.

The app isn't notarised by Apple, so macOS refuses it the first time. Open *System Settings
> Privacy & Security*, scroll down to the message about Chipload and choose *Open Anyway*.

- **Command line:** it's in `chipload-X.Y.Z-macos-universal.tar.gz`. Unpack it and put
  `chipload` on your `PATH`. If macOS stops it, run `xattr -d com.apple.quarantine chipload`
  once.
- **Opening files:** use *File > Open* in the app; double-clicking a `.chipload` file in
  Finder doesn't open it yet.

## From source

Install Rust with [rustup](https://rustup.rs), then:

```sh
git clone https://github.com/gntech/chipload
cd chipload
cargo build --release -p chipload-gui -p chipload-cli
# target/release/chipload-gui and target/release/chipload
```

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
