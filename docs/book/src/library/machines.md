# Machine profiles

A machine profile keeps a machine's values, so new projects start with them:
- the post-processor;
- safe Z, the travel, the max feed;
- the spindle direction;
- how tool changes are written;
- the work zero.

Profiles are kept in `machines.json` in your [settings folder](../install.md#your-settings).
As with tools, a project copies the values; the profile is only where new projects start.

## In the app

Your profiles are under **Library > Machines** in the [Browser](../window.md#the-browser).
**Machines** in the ribbon's LIBRARY group opens it.
- **Click a profile** to edit it in Properties; **Save** writes it.
- **Right-click** it for *Use in this project*, *Make default* and *Delete*.

In Job settings > Machine:
- **Use profile…** applies a profile to the project;
- a line says how the project's machine stands to the profile it came from, as for
  [tools](tools.md#project-tools-and-the-library), with *Update from library*, *Save to
  library* and *Show in library*;
- **Save as new…** (or **Save to library…**) saves the project's machine values as a profile.

A profile whose post is one of your own [post-processors](../posts.md#your-own-post) copies
that post into the project.

## On the command line

```sh
chipload machine list
chipload machine add "Shapeoko" --post grbl --safe-z 10 --travel 838,838,95 --max-feed 5000
chipload machine set "Shapeoko" --safe-z 15        # or --rename
chipload machine default "Shapeoko"
chipload import part.dxf -o part.chipload --machine "Shapeoko"   # start from a profile
chipload machine use "Shapeoko" part.chipload                   # apply to an existing project
chipload machine save part.chipload --name "Shapeoko 4"         # links the project to it
chipload machine compare part.chipload                          # same, differs, not in the library, or not from a profile
chipload machine export machines.json ; chipload machine import machines.json
chipload machine remove "Shapeoko 4"
```

`chipload machine add --help` lists every value. The [file format](../reference/machines-file.md)
is documented.
