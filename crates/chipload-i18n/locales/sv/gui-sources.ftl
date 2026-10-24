# The Browser's Drawing folder: the project's DXFs with their layers, its pictures, texts
# and drawn shapes (crates/chipload-gui/src/app/sources.rs).

# The folder's name: everything that is drawn, a noun.
sources-drawing = Ritning
sources-drawing-hover = DXF:er, bilder, texter och ritade former; högerklicka för att lägga till
sources-add-dxf = Lägg till DXF…
sources-add-dxf-hover = En till detalj: en DXF som placeras och uppdateras för sig
# "File > Import DXF" names a menu item.
sources-add-dxf-disabled-hover = Arkiv > Importera DXF… startar ett projekt från en
sources-import-image = Importera bild…
sources-import-image-hover = Spåra en bild till former, för V-gravyr
sources-add-text = Lägg till text
sources-add-text-hover = Skriv en text och klicka sedan där den börjar
sources-no-shapes = Inga former än.
# The name of DXF files in the file dialog's filter.
sources-dxf-filter = DXF-ritningar

# Rows of the folder. $file: the picture's file name; $text: what the text says.
sources-image-title = { $file } (bild { $id })
sources-text-title = { $text } (text { $id })
# Rows for the shapes drawn in the app, by kind.
sources-rectangles = Rektanglar
sources-circles = Cirklar
sources-polylines = Polylinjer
sources-other-paths = Övriga banor

sources-dxf-hover = Klick markerar hela DXF:en: en operation gjord från den följer dess uppdateringar. Läge och skala visas i Egenskaper; högerklicka för mer
sources-image-hover = Klick markerar hela bilden: en operation gjord från den följer när den spåras om. Storlek och läge visas i Egenskaper; dubbelklicka för spårningsinställningar
sources-text-hover = Klick markerar hela texten: en operation gjord från den följer dess ändringar. Innehåll, typsnitt, storlek och läge finns i Egenskaper
sources-drawn-hover = Klick markerar formerna; Ctrl+klick lägger till dem
sources-layer-hover = Klick markerar detta lager i denna DXF: en operation gjord från det fräser bara detta lager i DXF:en

# Right-click menu items, verbs.
sources-select = Markera
sources-show = Visa
sources-hide = Dölj
sources-place = Placera…
sources-place-hover = Flytta, vrid eller skala denna DXF, i dess egenskaper
sources-update-from-dxf = Uppdatera från en DXF…
sources-update-from-path = Uppdatera från en sökväg…
sources-remove-dxf-hover = Ta bort denna DXF och dess former
sources-trace-settings = Spårningsinställningar…
sources-trace-settings-hover = Tröskel, utjämning, konturer eller mittlinjer; storlek och läge finns i dess egenskaper
sources-remove-image-hover = Ta bort bilden och dess former
sources-remove-text-hover = Ta bort texten och dess bokstäver

# Followed by the project's problems, one per line.
sources-project-incomplete = Projektet är inte komplett:
# Followed by the error on its own line.
sources-remove-dxf-failed = Kunde inte ta bort DXF:en:
