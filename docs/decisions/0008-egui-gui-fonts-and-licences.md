# 0008: egui GUI: system fonts, BSL-1.0 allowed

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

## File dialogs
On Linux rfd uses the XDG desktop portal, falling back to `zenity`. Where neither exists
(e.g. WSL), *File → Open path…*, *Save G-code to path…* and command-line arguments do the
same job.

## Thin UI
As in Python, the window only calls `chipload-pipeline` (run_job, report text) and
`select_contours` for the selector preview; drawing, pan/zoom and picking are view code.
