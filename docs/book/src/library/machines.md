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

**Machines** in the ribbon's LIBRARY group lists the profiles and marks the default.

In Job settings > Machine:
- **Use profile…** applies a profile to the project;
- **Save as profile…** saves the project's machine values as a profile.

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
chipload machine save part.chipload --name "Shapeoko 4"
chipload machine export machines.json ; chipload machine import machines.json
chipload machine remove "Shapeoko 4"
```

`chipload machine add --help` lists every value. The [file format](../reference/machines-file.md)
is documented.
