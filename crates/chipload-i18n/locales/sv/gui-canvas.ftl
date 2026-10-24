# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Återställ start
canvas-reset-start-disabled-hover = Slingan börjar där Chipload valde
# $id: the contour's number.
canvas-select-contour = Markera #{ $id }
canvas-select-layer = Markera lager { $layer }
canvas-right-click-hint = Högerklicka på en kontur för att markera dess lager.

# Hovering a ring's start marker.
canvas-start-moved-hover = Slingans start: dra den längs slingan; högerklicka för att återställa
canvas-start-hover = Slingans start: dra den längs slingan för att flytta den

# Hovering plunges: "Operation 2 (Pocket 1): plunges 3, 13, 23 of 30". $operation is the
# operation's label, $numbers the list of plunge numbers, $total how many it has.
canvas-plunges = { $operation }: { $count ->
        [one] nedmatning
       *[other] nedmatningar
    } { $numbers } av { $total }
canvas-rejected-toolpath = Underkänd verktygsbana

# The notice at the canvas's top.
canvas-updating-toolpath = Uppdaterar verktygsbanan…
canvas-toolpath-out-of-date = Verktygsbanan är inaktuell
# A button: generate the toolpath again.
canvas-generate = Beräkna
canvas-generate-hover = Projektet har ändrats sedan det beräknades (F5)
canvas-updating-simulation = Uppdaterar simuleringen…

# The legend in the bottom left corner.
canvas-legend = Teckenförklaring
canvas-legend-hover = Vad ritytan visar
canvas-show-toolpath = Visa verktygsbana
canvas-show-toolpath-hover = Dölj verktygsbanan för att se ritningen eller det simulerade ämnet
