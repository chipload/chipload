# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = DXF has no units; using { $units } as chosen
# $max is the largest height found.
dxf-nonzero-z = { $count } entities have nonzero Z (max |Z| { $max }); Z was dropped
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source }: { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Block nesting deeper than { $most }
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }): { $reason }
dxf-skip-coordinate = a coordinate is not a number or beyond { $most }
dxf-skip-fit-points = has fit points only; re-export it with control points
dxf-skip-not-flat = not parallel to the XY plane
# $block is the block's name, in quotes.
dxf-skip-no-block = block { $block } does not exist
# $radius is in the drawing's units, without one.
dxf-skip-radius = radius { $radius } is not positive
dxf-spline-degree = degree must be at least 1
dxf-spline-few-points = { $count } control points is too few for degree { $degree }
dxf-spline-knots = { $knots } knots, expected { $expected } for { $count } control points of degree { $degree }
dxf-spline-knots-order = knots are not ascending
dxf-spline-weights = weight count differs from control point count

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = Removed zero-length { $type } on layer { $layer } ({ $handle })
dxf-duplicate = Removed duplicate { $type } on layer { $layer } ({ $handle })
dxf-branch-point = More than two segments meet at one point
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = Contour { $contour } on { $layers } is open: ends at { $start } and { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Contour { $contour }: { $reason }
dxf-ring-few-points = fewer than 3 distinct points
dxf-ring-self-intersecting = self-intersecting
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = folds back on itself: its edges come within { $within } of each other near ({ $x }, { $y })
dxf-ring-no-area = encloses no area
