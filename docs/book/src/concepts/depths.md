# Depths and clearance

## Start and end depth

Every operation cuts from its **start depth** to its **end depth**, in mm down from the
stock top, whatever the [Z zero](stock.md#stock) is.

- **Rule:** 0 ≤ start depth < end depth.
- **Passes:** the cut goes down in equal passes no deeper than the **step down**, and the
  last pass is exactly at the end depth. Start 2, end 6, step down 2 gives passes at 4 and
  6 mm.
- **Cutting through:** an end depth below the stock bottom goes into the spoilboard. That's
  allowed up to *Max depth below stock* (Job settings), with a warning; deeper is refused,
  so a typo can't reach the machine bed. For 12 mm board, an end depth of 12.3 cuts
  0.3 mm into the spoilboard.
- **Above the start depth:** the tool goes down at the plunge feed all the way from safe Z,
  also through the air above the start depth. Chipload doesn't know whether another
  operation already cleared it.

## Clearance

**Clearance** moves the cut into the material that is kept, by that many mm. Positive
removes more; negative leaves material.

| Operation | Positive clearance | Negative clearance |
|---|---|---|
| Pocket | the pocket larger all round, islands smaller | the pocket smaller, islands larger |
| Profile, outside | the part smaller | the part larger: stock left for a later cut |
| Profile, inside (a hole) | the hole larger | the hole smaller |
| Profile, on the line | not allowed | not allowed |

- **How:** the tool's centre runs at its radius minus the clearance from the drawn line.
- **Limit:** a clearance of the tool radius or more is refused. Use a profile *on the
  line*, or a smaller tool.
- **Corners:** inside corners of the kept material stay as round as the tool. Outside
  corners get rounded with the clearance's radius, because the tool reaches past them.
- **Use:**
  - a little positive clearance gives a fit, such as a lid that drops into its box;
  - negative clearance leaves stock for a finishing pass (see also the profile's
    [finishing pass](../operations/profile.md#finishing-pass)).
