# Problems

The app checks the project while you edit, before anything is generated:
- **On the field:** a field with a problem turns red, e.g. an end depth above the start
  depth, an oversize as large as the tool radius, or a tool without a spindle speed.
  Hovering it says why.
- **On the operation:** its row in the Browser gets a red dot, and hovering it shows the
  first problem.
- **Above the form:** a strip lists them all; click one to go to its operation.

Generating is blocked until they're fixed; warnings don't block it. On the command line,
`chipload check-project part.chipload` lists the same problems with the path of each field
(`operations.1.end_depth`), and `chipload set` can fix it by that path ([Scripting](../scripting.md)).

## Problems and warnings

| | Blocks generating | Examples |
|---|---|---|
| **Problem** | yes | depths out of order; a cut deeper than *Max depth below stock*; a feed above the machine's max feed; a helix radius at least the tool radius; a V-bit without an angle; a pocket with a drill |
| **Warning** | no | a cut into the spoilboard; a tool of unknown centre-cutting with a plunge; material the pocket's rings miss; a tab that couldn't move off a corner; open contours skipped |

Warnings appear in the report under the canvas after generating, on the command line as
`warning: …` lines, and on the setup sheet under *Watch out*.

## Generating

- **F5** (**Generate** at the ribbon's right end) plans the toolpath, writes the G-code and runs the
  [checker](../concepts/checker.md).
- With *View > Update toolpath automatically* (on by default), this happens by itself half
  a second after each change, in the background. The app stays responsive, and a running
  generation can be cancelled: **Cancel** takes Generate's place meanwhile.
- **Out of date:** a toolpath on the canvas that no longer matches the project is drawn
  faded until the next generation.

Export (Ctrl+E) is only possible once the current project has generated and passed the
check.
