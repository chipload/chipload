# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Reset start
canvas-reset-start-disabled-hover = This ring starts where Chipload chose
canvas-turn-axis = ⟲ Turn axis
# $id: the contour's number.
canvas-select-contour = Select #{ $id }
canvas-select-layer = Select layer { $layer }
canvas-select-similar = Select similar
canvas-select-similar-hover = Circles of the same diameter on any layer, or shapes of the same size on this layer. Ctrl: add to the selection
canvas-select-inside = Select inside
canvas-select-inside-hover = The closed shapes inside this one: a part's pockets and holes. Ctrl: add to the selection
canvas-right-click-hint = Right-click a contour to select its layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Start of this ring: drag it along the ring; right-click to reset
canvas-start-hover = Start of this ring: drag it along the ring to move it
canvas-tab-hover = Tab: drag it along the ring; right-click to remove it or space the ring's tabs evenly
canvas-tab-hover-placed = Tab placed by hand: drag it along the ring; right-click to remove it or space the ring's tabs evenly
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = Not moved: { $reason }
canvas-tab-remove = Remove tab
canvas-tab-even = Place this ring's tabs again
canvas-tab-add = Add a tab here
canvas-corner-hover = Corner at { $at }, { $angle }°: { $gets }. Click to choose its own
canvas-corner-hover-own = Corner at { $at }, { $angle }°: { $gets }, its own choice. Click to change it
canvas-corner-hover-unreached = Corner at { $at }, { $angle }°: { $gets }, but the tool can't reach into it, so it stays round. Click to choose
canvas-corner-as-operation = As the operation ({ $gets })
canvas-corner-round = Round
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-bone along the longer wall
canvas-corner-tbone-shorter = T-bone along the shorter wall
canvas-corner-gets-round = round
canvas-corner-gets-dogbone = a dogbone
canvas-corner-gets-tbone-longer = a T-bone along the longer wall
canvas-corner-gets-tbone-shorter = a T-bone along the shorter wall

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation }: { $count ->
        [one] plunge
       *[other] plunges
    } { $numbers } of { $total }
canvas-rejected-toolpath = Rejected toolpath

# The notice at the canvas's top.
canvas-updating-toolpath = Updating the toolpath…
canvas-toolpath-out-of-date = Toolpath out of date
# A button: generate the toolpath again.
canvas-generate = Generate
canvas-generate-hover = The project changed since it was generated (F5)
canvas-updating-simulation = Updating the simulation…

# The legend in the bottom left corner.
canvas-legend = Legend
canvas-legend-hover = What the canvas shows
canvas-show-toolpath = Show toolpath
canvas-show-toolpath-hover = Hide the toolpath to see the drawing or the simulated stock

## The Compass (docs/specs/compass.md).
compass-menu-item = Compass
compass-hover = Move along an arrow, turn with the ring, drag the centre onto a point; right-drag to copy or make an array; Space unlocks it to be put in place
compass-select-first = Select drawn shapes, a group, a picture, a text, a 3D model or a whole drawing first, then Q
compass-unlocked = Compass unlocked: drag it into place, Space locks it
compass-locked = Compass locked: a drag moves the selection
compass-typed-unreadable = Can't read that: type a length, an angle in degrees, or ΔX and ΔY
compass-move-here = Move here
compass-turn-here = Turn here
compass-copy-here = Copy here
compass-linear-array = Linear array…
compass-circular-array = Circular array…
compass-copies-drawn-only = Only drawn shapes can be copied
compass-status = Compass: drag an arrow, the ring or the centre; right-drag to copy; Space unlocks it; Q or Esc hides it
compass-status-arrow = Drag to move along this arrow; double-click to type the distance; right-drag to copy or make a linear array; right-click to move to a point
compass-status-ring = Drag to turn about the centre in 15° steps (Alt: 0.1°); double-click to type the angle; right-drag for a circular array
compass-status-centre = Drag to move freely, the centre snapping to points; double-click to type ΔX and ΔY; right-click to move to a point
compass-status-place = Drag to put the Compass on a point; Space locks it
compass-status-aim = Drag to turn the Compass's axes, or lay one along a line; Space locks it
compass-to-point = To point
compass-to-point-hover = Click a point next: the selection moves there (snapping as when dragging)
compass-to-centre = To centre point
compass-to-centre-hover = Click a shape next: the selection moves to its centre (a circle's or an arc's, else the middle of its box)
compass-no-centre-here = No shape there: click a shape to move to its centre (Esc cancels)
compass-status-pick-point = Click the point to move the selection to (Alt: no snapping; Esc cancels)
compass-status-pick-point-axis = Click a point: the selection moves along the arrow to it (Alt: no snapping; Esc cancels)
compass-status-pick-centre = Click a shape: the selection moves to its centre (Esc cancels)
compass-status-pick-centre-axis = Click a shape: the selection moves along the arrow to its centre (Esc cancels)
compass-value-distance = Distance
compass-value-angle = Angle
compass-value-delta = ΔX ΔY
# A part cut elsewhere than drawn: the label on the dashed box round its shapes
# (docs/specs/part-placement.md §4).
canvas-not-cut-here = { $part }: not cut here
