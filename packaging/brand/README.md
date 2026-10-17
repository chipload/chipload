# Chipload's logo

A **C made of a pocket's rings**, spiralling in the way Chipload cuts them, with the
**tool** as an orange dot where the cut starts. The wordmark puts that C, half again as tall
as the letters, before HIPLOAD in round-ended strokes. The look follows GNTECH's logo:
thick monoline strokes with round ends, and its sage green.

Everything is drawn from geometry by `make-brand.py`, as plain SVG paths, so no font is
needed to show it. Change the script, not the SVGs, then run `render.sh` (needs `resvg`:
`cargo install resvg --locked`) for the PNGs, the Windows icon and the copies the app,
packages and guide use.

## Files

| File | Use |
|---|---|
| `chipload-mark.svg`, `png/chipload-*.png` | the app icon: white rings on a green tile |
| `chipload-mark-green.svg`, `chipload-mark-white.svg` | the mark alone, on light or dark backgrounds |
| `chipload-wordmark.svg`, `png/chipload-wordmark.png` | mark and name, green: the README, the guide |
| `chipload-wordmark-white.svg` | the same in white, for dark backgrounds |
| `chipload-avatar.svg`, `png/chipload-avatar-512.png` | the GitHub organisation's picture (square, full bleed) |
| `chipload.ico` | the Windows icon (16 to 256 px), compiled into `chipload-gui.exe` |
| `github-social-preview.svg`, `png/github-social-preview.png` | GitHub's link picture (1280 x 640): repository *Settings > Social preview* |
| `github-profile/README.md` | the organisation's profile page: `profile/README.md` in the `chipload/.github` repository |

## On GitHub

- **Organisation picture:** `png/chipload-avatar-512.png` (organisation *Settings > Profile*).
- **Organisation profile:** name *Chipload*; description *2.5D CAM for CNC routers and mills:
  drawings in, checked G-code out*; website https://chipload.github.io/chipload/.
- **Repository:** the social preview above; description and website already set; topics
  cnc, cam, gcode, cnc-router, grbl, linuxcnc, mach3, dxf, v-carving, toolpath, rust, egui.

Copies: `packaging/linux/chipload.svg` (Linux and macOS icon), `crates/chipload-gui/assets/`
(window icon, Windows icon), `docs/book/theme/favicon.*` and
`docs/book/src/images/chipload-wordmark.svg` (the guide). In the app's top bar the mark is
drawn by `icons::paint_logo` from the same numbers.

## Colours

| | | |
|---|---|---|
| Green | `#9ab964` | the brand colour: the tile, the wordmark |
| Deep green | `#6f8f3c` | the mark on light backgrounds where the green is too pale |
| Orange | `#f29e38` | the tool, and only the tool: the one accent |
| Light | `#f3f5f0` | rings on the green tile, the wordmark on dark |
| Dark | `#1d2025` | dark backgrounds |

## Use

- Keep the orange dot: it is the tool, and it makes the C Chipload's.
- Don't stretch, outline, shadow or recolour the mark; on photos, use it on its tile.
- Leave room round it of at least the dot's diameter.
- Below 24 px use the tile, not the wordmark.

The logo is part of Chipload and under the same licence, GPL-3.0-or-later.
