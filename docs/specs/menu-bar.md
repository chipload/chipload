# Spec: a standard menu bar

Status: **built 2026-11-12**, decisions 1-6 as recommended (the user: "good, go ahead"). The user asked: "should we change the special file,
draw, view menus to standard menus?" They agreed with the proposal ("good, go ahead"), with
one correction: macOS builds ship too.

## 1. Why

Today the menus sit behind the small grey captions under the ribbon groups: FILE ▾,
DRAW ▾, MODIFY ▾, OPERATION ▾, WIZARDS ▾, LIBRARY ▾ and VIEW ▾. That came from
docs/specs/browser-and-layout.md §5 and its decision 6, to save one line of height. In use:

- **Hard to find.** Nobody expects a menu behind a caption. People coming from other CAD
  programs look for *File* at the top left first, and for keys such as Ctrl+S in its menu.
- **Scattered.** Help is an icon in the top bar. Units, the colour theme and the language
  are radio buttons in VIEW ▾, and again under *This computer > Preferences*. Undo and redo
  are in FILE ▾. Copy, paste and delete are in DRAW ▾.
- **Guide wording is awkward.** "DRAW ▾ > Frame…" reads oddly; "Draw > Frame…" is what
  everyone knows.

## 2. The menu bar, in the top bar

The menus go into the **top bar we already have**, after the quick icons, so the window
loses no height:

```
◆ Chipload 📂 💾 ↶ ↷   File  Edit  View  Draw  Modify  Operation  Wizards  Library  Help    [ sign.chipload ● ]     ◐  v0.35.0
```

- It uses egui's own menu bar, with the usual behaviour. A click opens a menu. While one is
  open, moving over another title opens that one. Esc or a click elsewhere closes it.
- **The project tab** stays in the top bar, centred in the room the menus leave. A long
  name is cut short with "…" and shown whole on hover.
- **Right end:** the theme button and the version. The Help icon goes, because *Help* is a
  menu now.
- **Narrow windows:** the project tab shortens its name, and goes when there is no room.
  The menu titles never shorten. The layout pictures (a narrow window, each language) check
  it.

## 3. The menus

Every item that exists today stays, under the menu where people look for it. The contents
come from today's functions, mostly unchanged.

| Menu | Holds | From today |
|---|---|---|
| **File** | New, Open, Open recent ▸, Add DXF or SVG…, Add picture…, Save, Save as…, Export G-code…, Export DXF…, Quit | FILE ▾, without undo and redo |
| **Edit** | Undo, Redo · Copy, Paste, Delete · Mirror ▸, Join ends, Construction, Copy as selector · Group, Ungroup, Add to group ▸, Remove from group, Save to shape library… · **Preferences…** | FILE ▾'s undo and redo, DRAW ▾'s shape items, the selection items |
| **View** | Fit, Top, 3D, Measure · Z scale ▸, Reverse left-right orbit · Grid, Rulers, Legend, Turn axis · Toolpath, Drawing, Pictures, Simulate · Panels ▸, Update automatically | VIEW ▾, without units, theme and language |
| **Draw** | Circle … Polygon, Text, Frame… | DRAW ▾'s drawing items |
| **Modify** | Offset … Polar array | MODIFY ▾ |
| **Operation** | the new-operation items, Inlay…, Apply style ▸ · Generate (or Cancel) | OPERATION ▾ |
| **Wizards** | New sign…, Two-sided… | WIZARDS ▾ |
| **Library** | Tools, Machines | LIBRARY ▾ |
| **Help** | User guide, Shortcuts, About, the update items | the Help icon |

(· marks a separator.)

**Preferences…** in Edit opens *This computer > Preferences* in the Browser. That page
already holds units, language, theme, the setup sheet and updating automatically, so those
items aren't repeated as radio buttons in a menu (decision 3).

## 4. The ribbon

- The captions under the groups become **plain labels**: FILE, DRAW and so on, with no ▾
  and no menu. Each menu then has one place, the menu bar.
- The buttons stay as they are.

## 5. macOS

- **For now, the menu bar is drawn in the window on every platform**, macOS too: egui draws
  it itself. It behaves the same everywhere and is tested headless as the rest of the window
  is.
- **The shortcuts in the menus show the Mac's keys on a Mac**: Cmd+S rather than Ctrl+S.
  Today `with_keys` always writes "Ctrl", while egui already maps Ctrl to Cmd on a Mac.
  Written as words, as egui's own names do: the ⌘ sign may be missing from the fonts.
- **Later, its own step:** the menus in the macOS menu bar at the top of the screen, with
  an app menu (About, Preferences…, Quit) as Mac users expect. That needs a crate, such as
  `muda` (MIT or Apache-2.0), wired into eframe's event loop, and a Mac to build and try it
  on. It gets its own ADR (decision 5).

## 6. Keyboard

The shortcuts work as today and are shown in the menus. Alt plus a letter to open a menu
(Alt+F for File) isn't built into egui. It is left for later (§8).

## 7. Guide

- *The window*: the top bar with its menus, and the ribbon table with its captions as labels.
- "FILE ▾ > …", "DRAW ▾ > …" and the like become "File > …", "Draw > …" throughout.
- The screenshots are renewed before the next release.

## 8. Not now

- The native macOS menu bar (§5, decision 5).
- Alt + letter to open a menu, and underlined letters.
- Menus that the user can arrange, and recently used items.

## 9. Decisions

1. **The menu bar goes in the top bar** (§2), between the quick icons and the project tab.
   *Recommended:* no height lost. Alternative: a line of its own above the top bar, about
   22 px more.
2. **Nine menus** (§3): File, Edit, View, Draw, Modify, Operation, Wizards, Library, Help.
   *Recommended:* they match the ribbon's groups, plus Edit and Help. Alternative: fold
   Modify into Draw and Library into File, for seven menus.
3. **Preferences… in Edit opens the Preferences page**, and units, theme and language leave
   the View menu (§3). *Recommended:* one place for each setting. *Reverse left-right orbit*
   stays in View beside Z scale, as it changes how you look, and is added to Preferences
   too. Alternative: a Settings menu with today's radio buttons.
4. **The ribbon captions become plain labels** (§4). *Recommended:* one way to each menu.
   Alternative: keep their drop-downs as a second way in.
5. **macOS: in-window menus now, the native menu bar later** with its own ADR (§5).
   *Recommended:* the native one needs a new dependency and a Mac to build and try it on.
   Alternative: build the native menu bar now, untested here.
6. **The menus show Cmd on a Mac** (§5). *Recommended:* a small fix, and right today too.

## 10. Settled while building

- **Names match the ribbon's captions** in every language (German *Bearbeitung*, Italian
  *Lavorazione*, French *Dessiner*, Russian *Черчение*). In Italian *Modifica* is Edit, as
  usual there, so the MODIFY group became *Trasforma*.
- **Draw's** adding a picture or a DXF went: they are in File.
- The language list in Preferences keeps the *(review wanted)* marks of the old menu.
