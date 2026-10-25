# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Språk
menu-language-system = System
menu-language-system-hover = Operativsystemets språk om Chipload har det, annars engelska
menu-language-review = { $language } (granskning önskas)
menu-language-review-hover = Översatt, ännu inte läst av någon med språket som modersmål som bearbetar själv. Rättelser är välkomna: se användarhandboken, Språk.

# File menu.
menu-new-project = Nytt projekt
menu-open = Öppna…
menu-open-recent = Öppna senaste
# Empties the list of recent projects.
menu-clear-list = Töm listan
menu-import-dxf = Importera DXF…
menu-import-image = Importera bild…
menu-import-image-hover = Spåra en bild till former, för V-gravyr
menu-open-path = Öppna sökväg…
menu-import-image-path = Importera bild från sökväg…
menu-several-dxfs = Flera DXF:er: uppdatera en från bläddraren (högerklicka på den)
menu-add-dxf = Lägg till DXF i projektet…
menu-add-dxf-hover = En till detalj: en DXF som placeras och uppdateras för sig
menu-add-dxf-path = Lägg till DXF från sökväg…
menu-save = Spara
menu-save-as = Spara som…
menu-save-path = Spara till sökväg…
menu-export = Exportera G-kod…
menu-export-path = Exportera G-kod till sökväg…
menu-quit = Avsluta

# Edit: undo, and the selected drawn shapes.
menu-undo = Ångra
menu-redo = Gör om
menu-only-drawn = Bara ritade former; importerade ändras bara med DXF:en
menu-copy = Kopiera
menu-paste = Klistra in
menu-mirror-left-right = Spegla vänster–höger
menu-mirror-up-down = Spegla upp–ner
menu-mirror-hover = Vänd de markerade ritade formerna kring mitten av deras ram
menu-delete-shapes = Radera former
menu-delete-shapes-hover = Raderar de markerade ritade formerna; importerade ändras bara med DXF:en
# A selector: the selection written as JSON, for scripts.
menu-copy-selector = Kopiera som selektor

# Job menu.
menu-cancel-generation = Avbryt beräkningen
menu-generate = Beräkna

# View menu.
menu-fit-view = Anpassa vy
menu-top-view = Vy uppifrån
menu-measure = Mät
menu-measure-hover = Två klick ger avståndet, ΔX, ΔY och vinkeln
menu-3d-view = 3D-vy
menu-z-scale = Z-skala i 3D
menu-show-grid = Visa rutnät
menu-show-grid-hover = Linjer på fästrutnätet, en kraftigare var tionde
menu-show-legend = Visa teckenförklaring
menu-show-browser = Visa bläddrare
menu-show-properties = Visa Egenskaper
menu-fold-panels = Fäll ihop båda panelerna
menu-open-panels = Öppna båda panelerna
menu-panels-hover = Största ritytan: bläddraren och Egenskaper ihopfällda till smala lister
menu-auto-update = Uppdatera verktygsbanan automatiskt
menu-auto-update-hover = Beräkna av sig själv en halv sekund efter varje ändring
menu-show-toolpath = Visa verktygsbana
menu-show-toolpath-hover = Dölj verktygsbanan för att se ritningen eller det simulerade ämnet
menu-show-drawing = Visa ritning
menu-show-drawing-hover = Dölj ritningen för att se bara ämnet eller verktygsbanan
menu-show-images = Visa bilder
menu-show-images-hover = Bilderna som spårats till former, svagt över ämnet
# The stock: the material being cut, as the simulation shows it.
menu-show-simulation = Visa ämne (simulering)
menu-simulation-hint = Visa ämnet som programmet lämnar det: varje rörelse fräst med verktygets form (mörkare är djupare; rött är under ämnet). Endast visning.
menu-units = Enheter
menu-units-mm-hover = Längder i mm, matning i mm/min
# The unit's name, as a choice next to "mm".
menu-units-inch = tum
menu-units-inch-hover = Längder i tum, matning i in/min; projekt förblir i mm
menu-theme = Färgtema
menu-theme-system = System
menu-theme-system-hover = Ljust eller mörkt, som operativsystemet är inställt
menu-theme-light = Ljust
menu-theme-light-hover = Ljusa paneler och rityta
menu-theme-dark = Mörkt
menu-theme-dark-hover = Mörka paneler och rityta

# Draw menu.
menu-draw-circle = Cirkel
menu-draw-rectangle = Rektangel
menu-draw-polyline = Polylinje
menu-draw-text = Text
menu-draw-text-hover = Skriv en text och klicka sedan där den börjar

# Help menu.
menu-user-guide = Användarhandbok
menu-shortcuts = Kortkommandon
menu-about = Om och licenser

# The status bar under the canvas. What the toolpath is of, with several parts:
menu-status-part-of = detalj { $part } av { $count }
menu-status-all-parts = alla detaljer
# $time: how long the program runs, e.g. "12 min 30 s".
menu-status-cycle = cykeltid { $time }
menu-status-time-hover = Uppskattad cykeltid (rapporten har detaljerna)
menu-status-active-part = aktiv detalj: { $name }
menu-status-active-part-hover = Nya operationer hamnar i den aktiva detaljen (bläddraren, högerklicka på en detalj)
menu-status-units-hover = Längder visas i dessa (Visa > Enheter)
# $step: the grid's step with its unit, e.g. "10 mm".
menu-status-grid = rutnät { $step }
menu-status-grid-hover = Rutnätets steg: klick under ritning fästs mot det (G döljer det)
menu-status-post-hover = Postprocessorn (Jobbinställningar > Maskin)
# Pieces in all, from every part that is cut.
menu-status-pieces = { $count ->
    [one] { $count } st
   *[other] { $count } st
}
menu-status-copies = { $count ->
    [one] { $count } kopia
   *[other] { $count } kopior
}
# The cursor in program coordinates (from the work zero), after the drawing's X and Y.
menu-status-work = (arbete X { $x }  Y { $y })
# $target: what the cursor snapped to, e.g. "end of #4".
menu-snapped-to = — fäst mot { $target }
menu-generating = Beräknar…
# $snapped: empty, or "— snapped to …" with a space before it.
menu-dragging = Drar till ({ $x }, { $y }){ $snapped } (Alt: ingen fästning; Shift på en mittpunkt böjer segmentet)
menu-status-hint = Klicka på en kontur för att markera den · Ctrl+klick lägger till eller tar bort · högerklick markerar dess lager · Esc rensar
menu-join-ends = Foga ihop ändar…
menu-join-ends-hover = Foga ihop öppna ritade polylinjer vars ändar nästan möts till en: de markerade, eller alla om ingen är markerad
menu-join-ends-disabled-hover = Det finns inga ritade former. En DXF:s former kan fogas ihop när den har brutits upp (högerklicka på den i bläddraren).
