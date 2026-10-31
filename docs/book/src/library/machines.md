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

Your profiles are under **This computer > Machines** in the [Browser](../window.md#the-browser).
**Machines** in the ribbon's LIBRARY group opens it.
- **Click a profile** to edit it in Properties; **Save** writes it.
- **Right-click** it for *Use in this project*, *Make default* and *Delete*.

In Machine:
- **Profile** names the project's profile in a dropdown; pick another to give the project
  its machine values. Its safe Z, work zero and max depth below the stock stay as they are.
- **While the project's machine is the same as its profile**, the machine's values (post,
  spindle, max feed, tool changes, travel) are shown greyed under **From the profile**.
  **Edit here** makes them editable for this project; it stays linked.
- **When they differ,** each value that differs has a ↶ button: hover it for the profile's
  value, click it to take it. *Update from library* takes them all.
- **Safe Z, the work zero and the max depth below the stock** belong to the job: they're
  always editable and never count as differing. A profile only gives a new project its
  first values for them; choosing another machine later keeps them.
- A line under the dropdown says how the machine stands to its profile, as for
  [tools](tools.md#project-tools-and-the-library), with *Update from library* and *Save to
  library* when they differ. **⋯** beside the dropdown has *Save as new…* and *Show in
  library*.

![The project's machine from the profile "Shapeoko": the same as in the library, its
machine values greyed under From the profile](../images/machine.png)

A profile whose post is one of your own [post-processors](../posts.md#your-own-post) copies
that post into the project.

## On the command line

```sh
chipload machine list
chipload machine add "Shapeoko" --post grbl --safe-z 10 --travel 838,838,95 --max-feed 5000
chipload machine set "Shapeoko" --safe-z 15        # or --rename
chipload machine default "Shapeoko"
chipload import part.dxf -o part.chipload --machine "Shapeoko"   # start from a profile
chipload machine use "Shapeoko" part.chipload                   # its machine values into an existing project
chipload machine save part.chipload --name "Shapeoko 4"         # links the project to it
chipload machine compare part.chipload                          # same, differs, not in the library, or not from a profile
chipload machine export machines.json ; chipload machine import machines.json
chipload machine remove "Shapeoko 4"
```

`chipload machine add --help` lists every value. The [file format](../reference/machines-file.md)
is documented.
