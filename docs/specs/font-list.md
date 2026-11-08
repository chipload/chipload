# Spec: the font list's search and recently used fonts

Status: built (2026-11-08; the user: "there is a search box but it does not work. also it
would be nice to have a 'recently used' fonts at the top of the list"). Builds on
docs/specs/system-fonts.md §6.

## 1. Why

- **The search did not work.** The text's *Font* list is a drop-down that closed on any
  click, a click into its search field too, so nothing could be typed into it.
- **The same few fonts are picked again and again**, and they sit among 17 built-in fonts
  and maybe hundreds on the computer.

## 2. The list

From the top:

1. **Search**, a field that takes the keys as soon as the list opens. It keeps only the
   fonts whose name has the typed letters, ignoring case and spaces at either end: in
   Recently used, Built in and On this computer. A style heading with no font left is
   hidden.
2. **Recently used:** the last fonts picked, most recent first, each in its own face (a
   computer font's sample is loaded for it). A computer font that is no longer there is
   left out; the heading only shows when there is one.
3. **Built in**, by style, as before.
4. **On this computer**, as before (system-fonts.md §6).

The list stays open while the search is typed or clicked into, and closes on a click
outside it or when a font is picked. Picking clears the search, so the list opens whole
next time.

## 3. Kept

- Picking a font from the list puts it at the top of the recently used fonts, moving it if
  it is there already; at most 5 (`RECENT_FONTS_MAX`).
- They are kept in the app settings (`settings.json`), field `recent_fonts`: names, most
  recent first, no duplicates, none empty. An optional field with a safe default (missing:
  none), so no version bump (CLAUDE.md, schema versioning).
- Only the window keeps them; the command line has no font list.

## 4. Tests

- `chipload-job` tests/library.rs: order, the limit, no duplicates; a list with a font
  twice is refused.
- `chipload-gui` tests/texts.rs: the list opens with the search focused; a click into it
  keeps the list open; typing "mono" keeps FreeMono and FreeMono Bold, hides Satisfy and
  the Script heading; picking FreeMono changes the text, closes the list and makes it the
  recently used font; opened again, the search is empty and *Recently used* shows.
