# The drawing canvas: its right-click menu, hover texts and notices (crates/chipload-gui/src/app/canvas.rs).

# The right-click menu. A ring's start: where a profile or pocket lap begins.
canvas-reset-start = Återställ start
canvas-reset-start-disabled-hover = Slingan börjar där Chipload valde
canvas-turn-axis = ⟲ Vändaxel
# $id: the contour's number.
canvas-select-contour = Markera #{ $id }
canvas-select-layer = Markera lager { $layer }
canvas-select-similar = Markera liknande
canvas-select-similar-hover = Cirklar med samma diameter på alla lager, eller former av samma storlek på detta lager. Ctrl: lägg till i markeringen
canvas-select-inside = Markera inuti
canvas-select-inside-hover = De slutna formerna inuti denna: en dels fickor och hål. Ctrl: lägg till i markeringen
canvas-right-click-hint = Högerklicka på en kontur för att markera dess lager.

# Hovering a ring's start marker.
canvas-start-moved-hover = Slingans start: dra den längs slingan; högerklicka för att återställa
canvas-start-hover = Slingans start: dra den längs slingan för att flytta den
canvas-tab-hover = Brygga: dra den längs slingan; högerklicka för att ta bort den eller sprida slingans bryggor jämnt
canvas-tab-hover-placed = Handplacerad brygga: dra den längs slingan; högerklicka för att ta bort den eller sprida slingans bryggor jämnt
# A dropped tab or ring start that the planner refuses goes back; $reason says why.
canvas-drop-refused = Inte flyttad: { $reason }
canvas-tab-remove = Ta bort bryggan
canvas-tab-even = Placera slingans bryggor igen
canvas-tab-add = Lägg till en brygga här
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

## Kompassen (docs/specs/compass.md).
compass-menu-item = Kompass
compass-hover = Flytta längs en pil, vrid med ringen, dra mitten till en punkt; högerdra för att kopiera eller göra ett mönster; mellanslag låser upp den så att den kan placeras
compass-select-first = Markera ritade former, en grupp, en bild, en text, en 3D-modell eller en hel ritning först, sedan Q
compass-unlocked = Kompassen upplåst: dra den på plats, mellanslag låser den
compass-locked = Kompassen låst: att dra flyttar markeringen
compass-typed-unreadable = Går inte att läsa: skriv en längd, en vinkel i grader, eller ΔX och ΔY
compass-move-here = Flytta hit
compass-turn-here = Vrid hit
compass-copy-here = Kopiera hit
compass-linear-array = Linjärt mönster…
compass-circular-array = Cirkulärt mönster…
compass-copies-drawn-only = Bara ritade former kan kopieras
compass-status = Kompass: dra en pil, ringen eller mitten; högerdra för att kopiera; mellanslag låser upp; Q eller Esc döljer den
compass-status-arrow = Dra för att flytta längs pilen; dubbelklicka för att skriva avståndet; högerdra för att kopiera eller göra ett linjärt mönster; högerklicka för att flytta till en punkt
compass-status-ring = Dra för att vrida runt mitten i steg om 15° (Alt: 0,1°); dubbelklicka för att skriva vinkeln; högerdra för ett cirkulärt mönster
compass-status-centre = Dra för att flytta fritt, mitten fäster vid punkter; dubbelklicka för att skriva ΔX och ΔY; högerklicka för att flytta till en punkt
compass-status-place = Dra för att sätta kompassen på en punkt; mellanslag låser den
compass-status-aim = Dra för att vrida kompassens axlar, eller lägga en längs en linje; mellanslag låser den
compass-to-point = Till punkt
compass-to-point-hover = Klicka sedan på en punkt: markeringen flyttas dit (med fäst som vid dragning)
compass-to-centre = Till mittpunkt
compass-to-centre-hover = Klicka sedan på en form: markeringen flyttas till dess mitt (en cirkels eller en bågs medelpunkt, annars mitten av dess ram)
compass-no-centre-here = Ingen form där: klicka på en form för att flytta till dess mitt (Esc avbryter)
compass-status-pick-point = Klicka på punkten markeringen ska flyttas till (Alt: utan fäst; Esc avbryter)
compass-status-pick-point-axis = Klicka på en punkt: markeringen flyttas längs pilen dit (Alt: utan fäst; Esc avbryter)
compass-status-pick-centre = Klicka på en form: markeringen flyttas till dess mitt (Esc avbryter)
compass-status-pick-centre-axis = Klicka på en form: markeringen flyttas längs pilen till dess mitt (Esc avbryter)
