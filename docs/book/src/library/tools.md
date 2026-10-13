# Tool library

Your tools, kept between projects in `tools.json` in your [settings folder](../install.md#your-settings).
A project **copies** the tools it uses, so its G-code never depends on the library: it cuts
the same on another computer, or after you change the library.

## In the app

*Settings > Tool library…* lists your tools.
- **Add, change and remove** tools there.
- **Mark one as the default:** new projects start with it.

In the project's tools (click *Tools* or a tool in the Browser):
- **Use library tool…** copies a library tool into the project.
- **Save to library…** saves a project tool to the library.
- **Update from library** appears when a project tool came from the library and the library
  now has other values.

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
chipload tool save part.chipload --tool 2 --name "3 mm downcut"
chipload tool remove "5 mm drill"
```

Cutting values come as a set: give all four, or none.

## Sharing

`chipload tool export tools.json [--name …]` writes your tools, or some of them, to a file.
`chipload tool import FILE` adds a file's tools:
- tools you don't have are added;
- tools with the same name and the same values are left alone;
- tools with the same name but other values are listed, and kept as yours unless you add
  `--replace`.

The app has the same under *Settings > Tool library > Import… / Export…*. The [file
format](../reference/tools-file.md) is documented, so a script can make one, e.g. from a
supplier's catalogue.
