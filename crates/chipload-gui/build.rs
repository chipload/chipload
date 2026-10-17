// SPDX-License-Identifier: GPL-3.0-or-later
//! On Windows, the icon Explorer shows for chipload-gui.exe (packaging/brand/). Elsewhere the
//! package carries the icon (AppImage, macOS app), and the window gets it at run time.

fn main() {
    println!("cargo:rerun-if-changed=assets/chipload.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/chipload.ico");
        if let Err(err) = resource.compile() {
            // A release without its icon is a mistake to stop at, not to ship.
            eprintln!("error: cannot add the icon to chipload-gui.exe: {err}");
            std::process::exit(1);
        }
    }
}
