# Spec: a tidier profile form (spiral ramp and leads)

Status: approved 2026-10-13 (the user: "go ahead and use spiral ramp").

## 1. Why

User request (2026-10-13): "The settings for profile is a bit messy. After step down we
should have a checkbox called helix ramp. Helix ramp rules out lead in/out I think. We only
want lead in/out if we do straight plunges? Helix ramp is the best solution."

For a profile, the *ramp* entry already spirals down along the contour
(docs/specs/profile-spiral.md), never steeper than the ramp angle and never more than one
step down of material per lap. With it, a lead only adds a lead-in at the start depth,
usually in the air. A lead is worth having with straight plunges: every plunge then goes
down in the waste and leaves no mark on the part.

## 2. The form

In the profile's **Cut** section, after **Step down**:
- **Spiral ramp** (checkbox): on is the ramp entry, off a straight plunge. Hover: "The
  tool spirals down along the contour, never steeper than the ramp angle and never more
  than one step down per lap: no plunge into the part. On long contours the step down
  decides the descent, on small ones the angle."
- **ramp angle**, under it, only while it is on.
- **Lead-in and lead-out**, only while it is off: "Plunge in the waste and arc onto the
  contour, so the plunge leaves no mark on the part."
- A project that has a spiral ramp **and** a lead keeps both: the lead row shows, saying
  that with a spiral ramp it only leads in. Untick it to drop the lead. Nothing is changed
  in the file without being asked.
- **A tool that can't plunge** (not centre-cutting): with the spiral off, a line under it
  says that a straight plunge needs a centre-cutting tool. The existing problem refuses
  generating. New profiles for such a tool start with the spiral on, as now.
- The profile's **Entry and lead** section goes. Pockets and follow paths keep theirs.

## 3. Decisions

1. **Name:** *Spiral ramp* (user's choice), not *helix*, which for pockets means down round
   a small circle.
2. **Step down stays:** each lap goes down by whichever is less, the step down or what
   the ramp angle allows on that lap's length.
3. **Leads only with straight plunges** in new settings. Existing spiral-plus-lead
   projects keep cutting the same, with no migration.
4. **No change to the file format or the G-code.**

## 4. Tests

- The form draws for a profile with the spiral on, off, and on with a lead.
- Ticking the spiral sets the ramp entry with the default angle, and unticking sets a
  plunge. Problems on `entry` and `lead` mark the Cut section.

## As built, revised 2026-10-15

The spiral ramp is chosen in an **Entry** combo (*spiral ramp* or *straight plunge*), the
same control pockets and follow paths have for the same field, instead of a checkbox. Its
place (right after the step down), the ramp angle row and the lead rule are as above.
