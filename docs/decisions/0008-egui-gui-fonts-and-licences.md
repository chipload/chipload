# 0008: egui GUI: system fonts, BSL-1.0 allowed (Inter built in since 2026-11-06)

Status: accepted (2026-10-09)

The Rust GUI (`chipload-gui`) uses `eframe` 0.36 (MIT OR Apache-2.0) with the glow
(OpenGL) renderer, and `rfd` 0.17 (MIT) for native file dialogs. eframe 0.36 needs Rust
1.95, so the workspace `rust-version` is now 1.95 (the code already needed 1.88).

## Licences (decisions 2026-10-09)
- **BSL-1.0 is allowed.** egui's Windows clipboard uses `clipboard-win` and `error-code`
  (Boost Software License 1.0, GPL-compatible per the FSF). Added to `deny.toml` and the
  CLAUDE.md list.
- **No bundled fonts.** egui's default fonts (`epaint_default_fonts`) are under OFL-1.1 and
  the Ubuntu Font Licence 1.0, outside our list. eframe is built without `default_fonts`, and
  the GUI loads installed fonts at startup: fontconfig (`fc-match`) first, then well-known
  paths on Linux, Windows and macOS. Without a usable font it exits with a message saying
  what to install.
- **Amended 2026-11-06: Inter is built in.** The user approved the SIL Open Font License 1.1
  for the window's typeface (docs/specs/look-and-feel.md §9). The OFL lets fonts be bundled
  and embedded with any software; the FSF counts it free but not GPL-compatible for merging
  into a program's code, and a font built into the binary as data is not that. Inter 4.1,
  Regular and SemiBold, is included unchanged (`crates/chipload-gui/fonts/`, its licence
  beside it and in THIRD-PARTY-LICENSES.txt); it declares no Reserved Font Name. The
  system's fonts are still loaded, as fallbacks for symbols, other scripts and monospace,
  and a missing one no longer stops the app. egui's own default fonts stay out.

## File dialogs
On Linux rfd uses the XDG desktop portal, falling back to `zenity`. Where neither exists
(e.g. WSL), *File → Open path…*, *Save G-code to path…* and command-line arguments do the
same job.

## Window frame on Wayland
eframe without its default features also drops winit's default `wayland-csd-adwaita`, leaving
winit's fallback client-side frame. Under WSLg's compositor that frame makes the connection
drop while resizing ("Io error: Broken pipe", then exit). `chipload-gui` therefore depends on
`winit` directly, only to enable `wayland-csd-adwaita` (sctk-adwaita, MIT; it draws the title
with system fonts). Checked by resizing the window 400 times in a row: fallback frame
disconnects, adwaita frame survives.

## Thin UI
As in Python, the window only calls `chipload-pipeline` (run_job, report text) and
`select_contours` for the selector preview; drawing, pan/zoom and picking are view code.
