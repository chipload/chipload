# Spec: the library in the Browser, and project tools linked to it

Status: approved 2026-10-14 (the user: "add id to 6 as well, better to do it right now, then
go ahead").

## 1. Why

User request (2026-10-14): "I am contemplating whether to put tool library / machine library
in the browser tree so that we see the tools as leaves under it and can select each and
edit. The thing now where the tool library exists but the tools in the project are copies is
a bit hard to understand."

A project keeps **copies** of the library tools and machine it uses (ADR 0014), so it cuts
the same in a year and on another computer. That stays. What changes is that the link
between a copy and its library entry is shown, and kept by an id instead of by name.

## 2. Ids (decision 6)

- **Every library tool and machine profile gets a `uid`:** 16 lowercase hex digits, unique in
  its file. It never changes, also not when the entry is renamed. Tools and machines are
  separate files, so the same uid may appear in both.
- **A new entry's uid** is worked out from its name: FNV-1a 64 of the lower-cased name. If
  that is taken in the file, from the name plus "#2", "#3" … So it's deterministic, with no
  clock and no random numbers.
- **A project tool and the project's machine get `library_uid`:** the uid of the entry they
  were copied from, or null. Like `name` (which stays, as the entry's name when copied), it is
  for reference only: planning and G-code never read it.
- **Formats:** `tools.json` 2 → 3 and `machines.json` 2 → 3, each entry getting the uid of
  its name; projects 22 → 23, each named tool and a named machine getting the uid of its name.
  Since both use the same rule, existing projects link to the entries they came from.
  Migrations stay pure.
- **Import into a library** keeps the file's uids. A new entry whose uid is taken by another
  entry gets a fresh one. A replaced entry keeps the library's uid, so your projects stay
  linked.
- **Export** writes the uids, so a library moved to another computer keeps projects linked.

## 3. The link

For a project tool (and the same for the machine):

| State | When | Shown |
|---|---|---|
| **Same** | the entry with its `library_uid` has the same values | "from library tool *6 mm 2F*" |
| **Differs** | that entry has other values | "differs from *6 mm 2F*: diameter 6 mm → 8 mm" with **Update from library** |
| **Missing** | no entry has that uid | "library tool *6 mm 2F* is not in your library" |
| **None** | `library_uid` is null | "not from your library" |

- The values compared are the ones a project copy has (the tool's number, name and cutting
  values aside; for a machine, the name aside and the max overcut included).
- **Update from library** copies the entry's values and its current name; the tool keeps its
  number. One undo step.
- **Save to library** writes the project's values into its linked entry (keeping its uid and
  name), or into a new entry under a name you give. The project tool is then linked to it.
- **Cutting values** for a new operation come from the linked entry while the tool is
  *Same*, as today by name.

## 4. The Browser

Above the project, a second root, **Library** ("this computer"), folded at the start (moved
to the top at the user's request, 2026-10-14):

- **Tools:** each library tool as a leaf ("6 mm 2F · endmill 6 mm"), *default* marked.
- **Machines:** each profile as a leaf ("Shapeoko · grbl, 800 × 800 × 80 mm").

A project tool's row shows its link at the right: "← 6 mm 2F", "← 6 mm 2F (changed)" when it
*Differs*, "(not in library)" when *Missing*. Hover gives the state's line of §3.

**Clicks:**
- **A library tool or machine** shows it in Properties (§5).
- **Library > Tools or Machines** shows the folder: where the file is, its problems, *New*,
  *Import…* and *Export…*, and an import's preview.
- **Right-click a library tool:** *Add to project*, *Make default*, *Delete*. **A machine:**
  *Use in this project*, *Make default*, *Delete*. **A folder:** *New*, *Import…*, *Export…*.
- **Drag a library tool** onto the project's *Tools* row (or any of its tools) to add it to
  the project, as *Add to project*.
- **Without a settings folder** the Library root says so and has no leaves.

## 5. Properties

- **A library tool:** its name, tool number, values and cutting values, as the Tool library
  window has them, then *Save* and *Revert* (only while changed), *Add to project*, *Make
  default* and *Delete*. Edits stay a draft until *Save*, which writes `tools.json`. A draft
  is kept while you look at other rows, until *Save* or *Revert*.
- **A machine profile:** the same, with the machine's rows and *Use in this project*.
- **A project tool** shows the link of §3 at the top, with *Update from library*, and, when
  linked, *Save to library*, *Save as new…* and *Show in library* (else *Save to
  library…*).
- **Job settings** shows the machine's link the same way.

## 6. What goes

- **The Tool library and Machines windows** go: the Browser and Properties do it all.
- **Ribbon LIBRARY > Tools / Machines** and their menu items open the Library root and show
  that folder.

## 7. Command line

- `tool list` and `machine list` show each entry's uid (`--json` too).
- `tool set --rename` and `machine set --rename` keep the uid, so projects stay linked.
- `tool use`, `tool save`, `machine use`, `machine save` set the project's `library_uid`.
- **New:** `chipload tool compare part.chipload` and `chipload machine compare part.chipload`
  print each project tool's (or the machine's) link state of §3, with the differences.

## 8. Decisions

1. **The library is a second root in the Browser,** above the project, folded at the start. *Approved.*
2. **Library entries are edited in Properties;** the windows go. *Approved.*
3. **Project tools show their link,** with *Update from library* and *Save to library*.
   *Approved.*
4. **Drag or right-click adds a library tool to the project.** *Approved.*
5. **The machine works the same way.** *Approved.*
6. **Links are by uid,** with schema bumps for projects (23), tools (3) and machines (3).
   *As the user asked.*
7. **An entry's edits are a draft until Save,** since they write a file other projects use.
   *Recommended.*
8. **A new uid comes from the name,** not from random numbers or the clock: migrations and
   saves stay deterministic, and a migrated project links to the entry migrated from the
   same name. *Recommended.*

## 9. Tests

- **Formats:** tools/machines v2 files migrate with the uid of each name; a v22 project
  migrates its named tools and machine to the same uids, and its G-code is unchanged (golden).
  Duplicate or malformed uids are refused in a library.
- **Uids:** a rename keeps the uid; a new entry whose name hash is taken gets "#2"'s; import
  keeps uids and replaces a clashing one.
- **Link:** Same, Differs (with changes), Missing and None, for tools and the machine.
- **CLI:** `tool compare` prints the four states; `tool set --name` keeps a project linked.
- **App:** focus on a library tool, edit, Save writes the library; Revert drops the draft;
  *Add to project* links the copy; Update from library copies values and name.
