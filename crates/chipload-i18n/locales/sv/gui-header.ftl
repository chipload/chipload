# The header: the top bar and the ribbon of grouped buttons (crates/chipload-gui/src/app/header.rs).

# Export's hover text. $count is the number of parts that are cut (always more than one),
# $part the name of the part the preview shows.
header-export-hover = Spara den kontrollerade G-koden
header-export-hover-parts = Spara den kontrollerade G-koden för varje detalj som fräses ({ $count } detaljer)
header-export-hover-parts-shown = Spara den kontrollerade G-koden för varje detalj som fräses ({ $count } detaljer); förhandsvisningen visar bara "{ $part }"
header-export-generating-hover = Spara G-koden när den har beräknats
header-export-first-hover = Beräkna först

# The top bar's icon buttons.
header-open-hover = Öppna ett projekt
header-save-hover = Spara projektet
header-undo = Ångra
header-redo = Gör om
header-help = Hjälp
# The colour theme button: what is set now, a click goes on to the next.
header-theme-system-hover = Färgtema: som systemet. Klicka för nästa.
header-theme-light-hover = Färgtema: ljust. Klicka för nästa.
header-theme-dark-hover = Färgtema: mörkt. Klicka för nästa.

# The tab with the open project's file name.
header-new-project = Nytt projekt
header-tab-unsaved-hover =
    { $path }
    Osparade ändringar
header-tab-not-saved-hover = Inte sparat än

# The ribbon's groups: their captions (shown in capitals) open the menu of the same name.
header-group-file = Arkiv
header-group-draw = Rita
header-group-operation = Operation
header-group-library = Bibliotek
header-group-view = Visa
header-group-hover = Allt under { $group }

# FILE: short words under icons.
header-new = Nytt
header-new-hover = Nytt projekt
header-open = Öppna
header-import = Importera
header-import-hover = Nytt projekt från en DXF eller SVG
header-save = Spara

# DRAW: short words under icons, and what each does.
header-circle = Cirkel
header-circle-hover = Rita en cirkel: centrum, sedan en punkt på den
header-rectangle = Rektangel
header-rectangle-hover = Rita en rektangel: två motstående hörn
header-polyline = Polylinje
header-polyline-hover = Rita en polylinje: klicka punkter
header-text = Text
header-text-hover = Skriv en text: klicka där den börjar och skriv den sedan i Egenskaper
header-image = Bild
header-image-hover = Spåra en bild till former, för V-gravyr eller Följ bana
header-add-dxf = Lägg till DXF
header-add-dxf-hover = En till detalj: en DXF som placeras och uppdateras för sig
# "FILE > Import": the ribbon group FILE and its Import button.
header-add-dxf-first-hover = Importera en DXF eller SVG först (ARKIV > Importera)
header-import-image-menu = Importera bild…
header-add-dxf-menu = Lägg till DXF eller SVG i projektet…

# OPERATION: "Follow" is the short word under the icon for a Follow path operation.
header-follow = Följ
# $operation is what the operation does (a hover text of its own).
header-needs-selection-hover = { $operation }. Markera former först.
header-select-shapes-first = Markera former först
header-new-profile = Ny konturfräsning
header-new-pocket = Ny ficka
header-new-vcarve = Ny V-gravyr
header-new-follow-path = Ny Följ bana
header-new-holes = Nya hål
header-new-surface = Ny planfräsning

# LIBRARY
header-tools = Verktyg
header-tools-hover = Ditt verktygsbibliotek, under Den här datorn i bläddraren
header-machines = Maskiner
header-machines-hover = Dina maskinprofiler, under Den här datorn i bläddraren
header-tool-library = Verktygsbibliotek

# VIEW
header-measure = Mät
header-measure-hover = Två klick ger avståndet, ΔX, ΔY och vinkeln. Det som visas finns på ritytans list.

# Generate and Export at the ribbon's right end.
header-export = Exportera
header-cancel = Avbryt
header-cancel-hover = Avbryt beräkningen
header-generate = Beräkna
header-generate-hover = Planera verktygsbanorna, skriv och kontrollera G-koden
header-arc = Båge
header-arc-hover = Rita en båge: start, slut, sedan en punkt den går genom
header-slot = Spår
header-slot-hover = Rita ett spår: mittpunkterna i ändarna, sedan bredden
header-polygon = Polygon
header-polygon-hover = Rita en regelbunden polygon: centrum, sedan ett hörn (antalet sidor i statusraden)
header-group-modify = Ändra
header-offset = Förskjut
header-offset-hover = En form på avstånd från de markerade: gå till den sida du vill, klicka eller skriv avståndet
header-fillet = Avrunda
header-fillet-hover = Runda ett hörn på den markerade formen med en båge: klicka hörnet; Skift+klick rundar alla hörn
header-chamfer = Fasa
header-chamfer-hover = Skär av ett hörn på den markerade formen rakt: klicka hörnet; Skift+klick fasar alla hörn
header-trim = Trimma
header-trim-hover = Ta bort biten av en ritad form under markören, fram till där andra former korsar den: klicka biten
header-extend = Förläng
header-extend-hover = Förläng en öppen ritad forms raka ände tills den möter nästa form: klicka nära änden
header-break = Bryt
header-break-hover = Dela en ritad form vid punkten du klickar: en öppen blir två, en sluten öppnas där
header-union = Förena
header-union-hover = Slå ihop de valda slutna formerna till en: allt som någon av dem täcker
header-subtract = Subtrahera
header-subtract-hover = Skär ut de valda slutna formerna ur formen du klickar på sedan
header-intersect = Snitt
header-intersect-hover = Behåll det som alla valda slutna former täcker
header-array = Matris
header-array-hover = Kopior av de valda ritade formerna i ett rutnät: kolumner, rader och avstånd under Markering
header-polar-array = Polär matris
header-polar-array-hover = Kopior av de valda ritade formerna runt en mittpunkt du klickar: antal och vinkel under Markering
