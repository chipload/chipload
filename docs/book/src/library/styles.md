# Styles

A **style** is one of your operations saved by name, to use again on other shapes in one
go: "Cut out 18 mm ply", "Pocket 6 mm", "Drill 5 mm". CamBam users know these as machining
styles.

A style keeps the operation's settings: side, direction, depths (a number, or
[*Through*](../concepts/depths.md#start-and-end-depth), which follows the next project's
stock), tabs, finishing pass, onion skin, corners, entry, hole method, V-carve passes, rest
and floor passes, and the [library tool](tools.md) it cuts with. It doesn't keep the shapes,
start points, tabs or corners placed by hand, or the operation's name and part.

**Feeds come from the tool.** A style keeps no feeds or step downs of its own: when it is
used, they come from its tool's [cutting data](tools.md#cutting-data) for the setup's
material, so one style works in every material your tool has values for, and a better feed
saved in the tool reaches every style that uses it. Only a tool without cutting data cuts
with the feeds the operation had when it was saved.

## Saving a style

Set an operation up as you want it, in any project, then save it under a name: right-click
it in the Browser, or the button under its properties, **Save as style…**, and give the
name (a name you already have replaces that style). On the command line:

```
chipload style save part.chipload --op 3 --name "Cut out 18 mm ply"
chipload style save part.chipload --op 3 --name "Cut out 18 mm ply" --replace   # in place of the old one
```

- **Its tools must be in your library:** the operation's, and its rest or floor passes'.
  A project tool that isn't is refused with "tool 2 isn't in your library: save it to the
  library first".
- **A profile's side** is kept as "by the selection": the style cuts outlines outside and
  holes inside, as a [new profile](../concepts/depths.md#what-a-new-operation-starts-with)
  does. A profile *on* the line keeps that.
- **To change a style,** use it, change the operation, and save it again with `--replace`.

## Using a style

Select the shapes on the canvas, then one of:
- **Style** at the end of the ribbon's OPERATION group: its menu lists your styles;
- right-click the canvas, **Apply style ▸**;
- right-click the style under *This computer > Styles*, or open it there: **Apply to the
  selection**.

On the command line:

```
chipload style use part.chipload "Cut out 18 mm ply" --dxf 1       # also --layer, --shapes, --group
```

- **The operations** go at the end of the active part, as a new operation of the kind would,
  with the style's settings: a profile style on an outline with its holes gives two, *Cut
  out 18 mm ply (holes)* inside first, then *(outline)* outside with the style's tabs; the
  holes get no tabs.
- **The tool:** the project's copy of the style's library tool, or, when the project has
  none, the library tool copied in (the command says so).
- **The name** is the style's, numbered when the project already has it ("Pocket 6 mm 2").
- Nothing in the project remembers the style afterwards: changing the style later doesn't
  change operations made from it.

## Managing styles

**This computer > Styles** in the Browser lists them; hover over one for what it does.
Click one for its properties: what it does in a few words, its name to change
(**Rename**), **Apply to the selection** and **Delete** (also the Del key; the library has
no undo, `styles.json.bak` keeps the file as it was).

On the command line:

```
chipload style list                       # every style in a few words, and where the file is
chipload style show "Pocket 6 mm"         # one style (--json: all of it)
chipload style rename "Pocket 6 mm" "Pocket 6 mm MDF"
chipload style remove "Pocket 6 mm MDF"
chipload style export styles-backup.json  # --name NAME (repeatable) for some
chipload style import styles-backup.json  # --replace overwrites styles of the same name
chipload style set "Cut out 18 mm ply" --tool "6 mm 3F"   # another tool of your library
```

- **From another computer:** a style whose tool isn't in your library is imported with a
  note, and lists as *not in your library*. A project that already has a copy of that tool
  can use it; another is refused until you give the style a tool with `style set … --tool`.

## The file

Styles are kept in `styles.json` in the [settings folder](../install.md#your-settings), beside
`tools.json`. Each style's operation is stored as a project stores it, with the version of
the project format it was written in: when a later Chipload changes operations, the styles
are carried over by the same steps as projects, the old file kept as `styles.json.bak`. A
style that can't be carried over is named and left out; the others load.
