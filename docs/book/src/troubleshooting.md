# Troubleshooting

## Importing

| Message or symptom | What to do |
|---|---|
| Asked for mm or inch | The DXF doesn't record its units. Choose the ones it was drawn in. In CAD, set the drawing units (`$INSUNITS`) before exporting. |
| `invalid_contour`: a contour crosses or folds back on itself | Fix it in CAD: a closed contour must not cross itself. Often a line drawn twice with a tiny offset, or a polyline that doubles back. |
| `open_contour` on a shape that should be closed | Its ends don't meet within 0.001 mm. Join them in CAD (snap the end points), or use [Follow path](operations/path.md) if it really is a line. |
| `branch_point` | Three or more lines meet at one point, so Chipload can't tell which way a contour goes. Remove the extra line in CAD. |
| Text is missing | TEXT entities aren't read. Convert text to outlines (curves) in CAD. |
| A DWG file | Not supported. Save it as DXF. |

## Generating

| Message | What to do |
|---|---|
| *tool (diameter … mm) does not fit in the pocket* | The pocket is narrower than the tool everywhere. Use a smaller tool. |
| *the tool does not fit through a narrow part* | A warning: the pocket is cleared as separate areas, and material stays in the narrow part. A [rest pocket](operations/rest.md) with a smaller tool can clear it. |
| *uncut material* | The pocket's stepover is too large for its shape. Lower the stepover. |
| *end depth … is … below the stock …, more than the allowed* | The cut is deeper than *Max depth below stock* allows. Check the stock thickness, or raise the allowance (Job settings > Stock). |
| *Tool N has no spindle speed* | Set the tool's spindle speed (rpm). |
| *a v-bit with no angle* | Set the V-bit's angle: click the tool in the Browser. |
| *clearing was removed in Chipload 0.14* | The pocket used trochoidal or adaptive clearing. Switch it to rings (**Use rings**). |
| *shapes … no longer exist; choose its geometry again* | After updating the DXF, an operation's shapes are gone. Select new ones, then *Use selection*. |

## The check failed

The [checker](concepts/checker.md) refused the program, so nothing was exported. The
message names the line and the rule:
- **Rapid into material:** usually the stock's thickness or *Z zero* is set wrong for the
  depths, or the safe Z is below the stock top.
- **Machine travel:** the program is larger than the machine's travel. Check the work
  zero and the drawing's position.
- **With your own post:** a code it writes that the checker doesn't accept. See
  [Post-processors](posts.md#your-own-post).

## The part comes out the wrong size

- **Units:** a drawing in inches read as mm (or the other way) is 25.4 times off. Check the
  units in the import report.
- **Oversize:** a positive oversize (`clearance` in the file) makes parts smaller and holes
  larger. See [Depths and oversize](concepts/depths.md#oversize).
- **Side:** an outside profile on a hole, or the other way, is off by a tool diameter.
  Check each profile's side.
- **Tool diameter:** Chipload offsets by the diameter you set. Measure the real tool.
