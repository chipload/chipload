# Tool library

Your tools, kept between projects in `tools.json` in your [settings folder](../install.md#your-settings).
A project **copies** the tools it uses, so its G-code never depends on the library: it cuts
the same on another computer, or after you change the library.

## In the app

Your tools are under **This computer > Tools** at the top of the [Browser](../window.md#the-browser). **Tools** in the ribbon's LIBRARY group opens it.
- **Click a tool** to edit it in Properties. Changes are a draft, marked *unsaved* in the
  Browser, until you press **Save**; **Revert** drops them.
- **Drag a tool** onto the project's *Tools* (or right-click it > *Add to project*) to copy it
  into the project.
- **Right-click** a tool for *Make default* (new projects start with it) and *Delete*, or
  *This computer > Tools* for *New tool*, *Import…* and *Export…*.

![This computer > Tools in the Browser, a library tool in Properties, and the project's tool T3
linked to it](../images/library.png)

### Project tools and the library

Each project tool remembers the library tool it was copied from, by an id that stays when
the library tool is renamed. Its row in the Browser shows that tool's name ("← 6 mm 2F"),
and its properties say how they stand:

| It says | Meaning | You can |
|---|---|---|
| from library tool "6 mm 2F" | same values | *Save to library*, *Show in library* |
| differs from "6 mm 2F": rpm 18000 -> 16000 | one of them was changed | *Update from library* (copies the library's values into the project), or *Save to library* (the other way) |
| "6 mm 2F" is not in your library | deleted there, or the project comes from another computer | *Save to library…* under a name |
| not from your library | made in the project | *Save to library…* under a name |

- **Use library tool…** gives the project tool another library tool's values.
- **Save as new…** keeps the project tool's values under another name.
- A new operation gets a library tool's cutting values while the project tool still has the
  library tool's values.

## What a tool has

The project tool's values ([Tools and tool changes](../concepts/tools.md)), plus:
- **Name:** how you find it, e.g. "6 mm 2F upcut".
- **Tool number:** the id it gets in a project, when that number is free there.
- **Flutes:** used for the [chip load](../checking/time.md#chip-load).
- **Cutting values** (optional): feed, plunge feed, step down and stepover. A new operation
  made with this tool starts with them. They never change an existing operation.

## On the command line

```sh
chipload tool list                                     # the tools, and where the file is
chipload tool add "6 mm 2F" --diameter 6 --flutes 2 --rpm 18000 \
  --feed 1500 --plunge-feed 400 --step-down 2 --stepover 0.4
chipload tool add "V 60" --type vbit --diameter 12.7 --angle 60 --tip 0.2
chipload tool add "5 mm drill" --type drill --diameter 5 --rpm 3000
chipload tool set "6 mm 2F" --rpm 20000                # or --rename
chipload tool default "6 mm 2F"
chipload tool use "V 60" part.chipload                 # copy into a project; prints its id there
chipload tool save part.chipload --tool 2 --name "3 mm downcut"   # links tool 2 to it
chipload tool compare part.chipload                # each project tool: same, differs, not in the library, or not from it
chipload tool remove "5 mm drill"
```

Cutting values come as a set: give all four, or none.

## Sharing

`chipload tool export tools.json [--name …]` writes your tools, or some of them, to a file.
`chipload tool import FILE` adds a file's tools:
- tools you don't have are added;
- tools with the same name and the same values are left alone;
- each tool keeps its id, so projects made on the other computer stay linked;
- tools with the same name but other values are listed, and kept as yours unless you add
  `--replace`.

The app has the same: right-click *This computer > Tools* for *Import…* and *Export…*. The [file
format](../reference/tools-file.md) is documented, so a script can make one, e.g. from a
supplier's catalogue.
