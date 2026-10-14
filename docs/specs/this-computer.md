# Spec: "This computer" in the Browser: preferences, library and posts

Status: approved 2026-10-14 (the user: "agree").

## 1. Why

User question (2026-10-14): "Should we add a general settings node to the Browser, where we
can put stuff that doesn't belong in the job settings?"

What is left over is all about this computer, not the project: the setup sheet setting
(for every project, yet shown in Job settings), *update the toolpath automatically*, the colour
theme, and where the settings folder and the posts are.

## 2. The Browser

The root above the project, *Library*, is renamed **This computer**:

- **Preferences** (a leaf): the app settings (§3).
- **Tools** and **Machines:** as now (docs/specs/library-in-browser.md).
- **Post-processors:** the built-in posts, then yours from the posts folder; one that can't
  be read is marked with the red dot.

## 3. Preferences

In Properties:
- **Settings folder:** where it is, or that there is none and which variable sets it.
- **Setup sheet:** write one next to exported G-code (every project). It leaves Job settings.
- **Update the toolpath automatically** while editing (also in the View menu).
- **Colour theme:** System, Light, Dark (also the top-bar button).

The grid and other view switches stay on the canvas bar.

## 4. Post-processors

- **The folder** shows where your posts go and how to make one (two files, see the guide),
  and how many there are.
- **A post** shows its name in the file, whether it is built in or yours (with its two
  files), the file extension, decimals, arcs, the tool change modes it writes and the
  spindle dwell, or why it can't be used. **Use in this project** makes it the project's
  post, as the Post box does.
- **Command line:** `chipload post show NAME` prints the same.

## 5. Decisions

1. **One root, "This computer",** for everything kept on this computer. *Approved.*
2. **Preferences** gets the setup sheet, update automatically and the theme; the setup sheet
   leaves Job settings. *Approved.*
3. **Post-processors** are listed, each with its facts and *Use in this project*.
   *Approved.*
4. **No format change.**
