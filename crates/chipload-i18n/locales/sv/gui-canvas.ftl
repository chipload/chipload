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
canvas-corner-hover = Hörn vid { $at }, { $angle }°: { $gets }. Klicka för att välja något eget
canvas-corner-hover-own = Hörn vid { $at }, { $angle }°: { $gets }, eget val. Klicka för att ändra
canvas-corner-hover-unreached = Hörn vid { $at }, { $angle }°: { $gets }, men verktyget når inte in i det, så det blir runt. Klicka för att välja
canvas-corner-as-operation = Som operationen ({ $gets })
canvas-corner-round = Runt
canvas-corner-dogbone = Dogbone
canvas-corner-tbone-longer = T-bone längs den längre väggen
canvas-corner-tbone-shorter = T-bone längs den kortare väggen
canvas-corner-gets-round = runt
canvas-corner-gets-dogbone = en dogbone
canvas-corner-gets-tbone-longer = en T-bone längs den längre väggen
canvas-corner-gets-tbone-shorter = en T-bone längs den kortare väggen

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
