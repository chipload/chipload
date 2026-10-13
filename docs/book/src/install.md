# Installing

Chipload is two programs:
- `chipload-gui`, the desktop app;
- `chipload`, the command line.

Both read and write the same project files, and both make the same G-code.

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
  Open path…*, or give the files on the command line.

## Windows

Download `chipload-gui-X.Y.Z-windows-x86_64.exe`, the app as one file with nothing to
install, and `chipload-X.Y.Z-windows-x86_64.exe`, the command line.

The files aren't code-signed, so Windows SmartScreen may warn the first time. Choose *More
info > Run anyway*.

## From source

Install Rust with [rustup](https://rustup.rs), then in the source folder:

```sh
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
| macOS (built from source) | `~/Library/Application Support/chipload/` |
| Any | the folder in the environment variable `CHIPLOAD_CONFIG_DIR`, if set |

A project keeps its own copy of the tools, machine and post it uses, so it cuts the same on
any computer, whatever that computer's library holds.
