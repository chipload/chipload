# The menu bar and its menus (crates/chipload-gui/src/app/menu.rs).

menu-language = Språk
menu-language-system = System
menu-language-review = { $language } (granskning önskas)
# File menu.
menu-new-project = Nytt projekt
menu-open = Öppna…
menu-open-recent = Öppna senaste
# Empties the list of recent projects.
menu-clear-list = Töm listan
menu-import-dxf = Importera DXF eller SVG…
menu-import-image = Importera bild…
menu-import-image-hover = Spåra en bild till former, för V-gravyr
menu-open-path = Öppna sökväg…
menu-import-image-path = Importera bild från sökväg…
menu-several-dxfs = Flera ritningsfiler: uppdatera en från bläddraren (högerklicka på den)
menu-add-dxf = Lägg till DXF eller SVG i projektet…
menu-add-dxf-hover = En DXF eller SVG i det här projektet, bredvid det som finns: placeras och uppdateras för sig
menu-add-dxf-path = Lägg till DXF eller SVG från sökväg…
menu-save = Spara
menu-save-as = Spara som…
menu-save-path = Spara till sökväg…
menu-export = Exportera G-kod…
menu-export-path = Exportera G-kod till sökväg…
menu-export-dxf = Exportera ritningen som DXF…
menu-export-dxf-hover = Formerna som ritytan visar, på sina lager, för en laserskärare, ett CAD-program eller en offert
menu-construction = Hjälpgeometri
menu-construction-hover = Hjälpgeometri: visas prickad och går att fästa mot, men fräses aldrig. För hjälplinjer och -cirklar att rita mot
menu-toggle-construction = Hjälpgeometri på eller av
menu-quit = Avsluta

# Edit: undo, and the selected drawn shapes.
menu-undo = Ångra
menu-redo = Gör om
menu-only-drawn = Bara ritade former; importerade ändras bara med sin fil
menu-copy = Kopiera
menu-paste = Klistra in
menu-mirror-left-right = Spegla vänster–höger
menu-mirror-up-down = Spegla upp–ner
menu-mirror-hover = Vänd de markerade ritade formerna kring mitten av deras ram
menu-delete-shapes = Radera
menu-delete-shapes-hover = Raderar det markerade: ritade former, eller en text, en bild eller en DXF i sin helhet; en enskild form ur en importerad ritning ändras bara med sin fil
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
menu-reverse-orbit = Vänd vänster-höger-rotation i 3D
menu-reverse-orbit-hover = Ett drag åt vänster eller höger flyttar vyn runt modellen, som Chipload gjorde förut, i stället för att vrida modellen med musen
menu-show-grid = Visa rutnät
menu-show-grid-hover = Linjer på fästrutnätet, en kraftigare var tionde
menu-show-rulers = Visa linjaler
menu-show-rulers-hover = Linjaler längs ritytans övre och vänstra kant i vy ovanifrån: ritningens koordinater, markören markerad
menu-show-turn-axis = Visa vändaxel
menu-show-turn-axis-hover = Linjen som en tvåsidig skiva vänds kring, streckad över skivan
menu-show-legend = Visa teckenförklaring
menu-fold-panels = Dölj panelerna
menu-open-panels = Visa panelerna
menu-panels-hover = Största ritytan: alla paneler dolda, och tillbaka som de var
menu-panels = Paneler
menu-reset-layout = Återställ layouten
menu-reset-layout-hover = Panelerna där de börjar: bläddraren till vänster, Egenskaper och Former till höger, Rapporten nederst
menu-auto-update = Uppdatera verktygsbanan automatiskt
menu-auto-update-hover = Beräkna av sig själv en halv sekund efter varje ändring
menu-show-toolpath = Visa verktygsbana
menu-show-toolpath-hover = Dölj verktygsbanan för att se ritningen eller det simulerade ämnet
menu-show-drawing = Visa ritning
menu-show-drawing-hover = Dölj ritningen för att se bara ämnet eller verktygsbanan
menu-show-images = Visa bilder
menu-show-images-hover = Bilderna som spårats till former, svagt över ämnet
# The stock: the material being cut, as the simulation shows it.
menu-simulate = Simulera
menu-simulation-hint = Visa ämnet som programmet lämnar det: varje rörelse fräst med verktygets form (mörkare är djupare; rött är under ämnet). Endast visning.
# The unit's name, as a choice next to "mm".
menu-units-inch = tum
# Draw menu.
menu-draw-circle = Cirkel
menu-draw-rectangle = Rektangel
menu-draw-polyline = Polylinje
menu-draw-text = Text
menu-draw-text-hover = Skriv en text och klicka sedan där den börjar
menu-draw-frame = Ram…
menu-draw-frame-hover = En ram för en skylt: runt markeringen eller i en storlek, med kantlinjer
menu-frame-selection = Ram runt markeringen…
menu-frame-selection-hover = En ram runt de markerade formerna, bara ritad: inga operationer. GUIDER > Skylt gör en hel skylt
menu-wizard-sign = Ny skylt…
menu-wizard-sign-hover = En hel skylt i ett steg: orden, en ram som följer dem och varje operation som skär den
menu-wizard-two-sided = Tvåsidig…
menu-wizard-two-sided-hover = En baksida för skivan med dess styrpinnar i ett steg, eller den baksida som finns ändrad

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
menu-status-post-hover = Postprocessorn (Maskin)
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
menu-dragging = Drar till ({ $x }, { $y }){ $snapped } (Alt: ingen fästning; på ett segment infogar Ctrl en punkt, Shift böjer det)
menu-status-hint = [Mellanslag+dra] flyttar vyn · [klick] markerar en ritning, bild, text eller grupp hel · [dubbelklick] en kontur · [Ctrl+klick] lägger till eller lämnar ute · [Esc] rensar
menu-status-simulate-hint = Simulera: [dra] för att panorera (3D: rotera) · klicka på en operations nummer för att öppna den · [F6] tillbaka till Redigera
menu-join-ends = Foga ihop ändar…
menu-join-ends-hover = Foga ihop öppna ritade polylinjer vars ändar nästan möts till en: de markerade, eller alla om ingen är markerad
menu-join-ends-disabled-hover = Det finns inga ritade former. En DXF:s former kan fogas ihop när den har brutits upp (högerklicka på den i bläddraren).
menu-draw-arc = Båge
menu-draw-slot = Spår
menu-draw-polygon = Polygon
menu-polygon-sides = Sidor
menu-modify-offset = Förskjut
menu-modify-fillet = Avrunda
menu-modify-chamfer = Fasa
menu-modify-trim = Trimma
menu-modify-extend = Förläng
menu-modify-break = Bryt
menu-modify-union = Förena
menu-modify-subtract = Subtrahera
menu-modify-intersect = Snitt
menu-modify-array = Matris
menu-modify-polar-array = Polär matris

menu-bar-file = Arkiv
menu-bar-edit = Redigera
menu-bar-view = Visa
menu-bar-draw = Rita
menu-bar-modify = Ändra
menu-bar-operation = Operation
menu-bar-wizards = Guider
menu-bar-library = Bibliotek
menu-bar-help = Hjälp
menu-preferences = Inställningar…
menu-preferences-hover = Den här datorn > Inställningar i Bläddraren: enheter, språk, färgtema, riggblad, uppdatera automatiskt
