# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Reset start
canvas-reset-start-disabled-hover = This ring starts where Chipload chose
# $id: the contour's number.
canvas-select-contour = Select #{ $id }
canvas-select-layer = Select layer { $layer }
canvas-right-click-hint = Right-click a contour to select its layer.

# Hovering a ring's start marker.
canvas-start-moved-hover = Start of this ring: drag it along the ring; right-click to reset
canvas-start-hover = Start of this ring: drag it along the ring to move it
canvas-tab-hover = Tab: drag it along the ring; right-click to remove it or space the ring's tabs evenly
canvas-tab-hover-placed = Tab placed by hand: drag it along the ring; right-click to remove it or space the ring's tabs evenly
canvas-tab-remove = Remove tab
canvas-tab-even = Space this ring's tabs evenly
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
