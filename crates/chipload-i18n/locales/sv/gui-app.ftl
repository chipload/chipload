# The main window: opening, importing, saving and exporting, parts, rest passes, the
# selection (crates/chipload-gui/src/app/mod.rs).

# The buttons of a folded side panel; P is the key that folds and unfolds both panels.
app-show-browser-hover = Visa bläddraren (P: båda panelerna)
app-show-properties-hover = Visa Egenskaper (P: båda panelerna)
# The heading of the right-hand panel.
app-properties = Egenskaper

app-start-hint = Importera en DXF eller SVG (Ctrl+I) för att starta ett projekt, eller öppna ett (Ctrl+O).

app-cannot-open = Kan inte öppna { $file }:
app-project = Projekt: { $file }
app-old-schema = Filen har schemaversion { $version }; när du sparar skrivs version { $current }.

app-new-project = Nytt projekt.
# $what: a machine or tool from the user's library, e.g. machine "Router".
app-started-with = Startade med { $what }.
app-new-project-next = Rita former från menyn Rita, eller hämta in en ritning med Lägg till DXF. Kontrollera ämne och maskin under Jobbinställningar, och verktyget under Verktyg.
app-units-missing = { $file } anger inte vilka enheter den använder; välj mm eller tum.
app-px-missing = { $file } anger sin storlek bara i pixlar; välj dess pixlar per tum.
app-cannot-import = Kan inte importera { $file }:
app-new-project-from = Nytt projekt från { $file } ({ $shapes } former).
app-import-next = Markera konturer och välj sedan Ny konturfräsning eller Ny ficka. Kontrollera ämne och maskin under Jobbinställningar, och verktyget under Verktyg.

# Followed by the list of problems.
app-project-incomplete = Projektet är inte komplett:
app-lost-geometry = Några operationer förlorade sin geometri: välj den igen
app-could-not-update-drawing = Kunde inte uppdatera ritningen:
app-could-not-add = Kunde inte lägga till { $file }:
app-could-not-move-drawing = Kunde inte flytta ritningen:

# The distance between copies of a part; $dy and $gap carry their unit.
app-pitch-set = Delning { $dx } × { $dy } ({ $gap } mellanrum)
app-pitch-needs-plan = Kunde inte sätta delningen från ett mellanrum: detaljen måste planeras först.

app-deleted-part = { $count ->
    [0] Raderade detaljen { $name } (Ctrl+Z tar tillbaka den)
    [one] Raderade detaljen { $name } och dess operation (Ctrl+Z tar tillbaka den)
   *[other] Raderade detaljen { $name } och dess { $count } operationer (Ctrl+Z tar tillbaka den)
}
app-part-needed = Ett projekt behöver minst en detalj

# A rest pass clears what a larger tool left in a pocket.
app-could-not-add-rest-pass = Kunde inte lägga till restbearbetning: { $error }
app-rest-with-corners = Fickan har dogbone- eller T-bone-hörn, och restbearbetning kan ännu inte kombineras med dem. Hörnen når redan dit en restbearbetning skulle nå.
# $tool: a tool number such as T3.
app-rest-tool-missing = Verktyg { $tool } finns inte i projektet: ge fickan ett verktyg först.
app-rest-no-smaller = Ingen pinnfräs mindre än { $tool } ({ $diameter }) i projektet eller ditt verktygsbibliotek. Lägg till en under Verktyg och högerklicka sedan på fickan igen.
app-now-rest-pass = Operation { $op } är nu restbearbetning av operation { $parent }; den fräses efter den följd av fickor den hör till

app-generate-before-export = Beräkna (F5) innan du exporterar G-kod.
app-planning-then-export = Planerar alla detaljer och exporterar sedan
app-planning-part-then-export = Planerar detaljen { $name } och exporterar sedan
app-could-not-export = Kunde inte exportera { $path }:
app-dxf-exported = Skrev { $path }.
app-could-not-export-dxf = Ritningen exporterades inte:
app-sheet-failed = G-koden exporterades, men uppsättningsbladet { $path } kunde inte skrivas:
app-exported-n-files = Exporterade { $count } filer: byt verktyg och nollställ Z mellan dem, men behåll nollpunkten i X och Y.
app-sheet-for-operator = Uppsättningsblad för operatören: { $path }
app-exported-files = Exporterade { $count } filer bredvid { $path }
app-exported-files-sheet = Exporterade { $count } filer och uppsättningsbladet { $sheet } bredvid { $path }
app-exported = Exporterade { $path }
app-exported-sheet = Exporterade { $path } och uppsättningsbladet { $sheet }

app-cannot-save-yet = Projektet kan inte sparas än:
app-could-not-save = Kunde inte spara { $path }:
app-saved = Sparade { $path }

# What is selected on the canvas.
app-nothing-selected = Inget markerat.
# $count contours selected; $what says how (one of the texts below); $list their numbers.
app-selected = { $count } markerade ({ $what }): { $list }
app-selection-layer = lager { $layer }
app-selection-dxf = DXF { $id }
app-selection-image = bild { $id }
app-selection-text = text { $id }
# $source: one of the three texts above.
app-selection-left-out = { $source }, { $count } utelämnade
app-selection-contours = konturer
# Open contours (not closed) are not cut.
app-open-not-cut = Öppna (fräses inte): { $list }

# File menu, for a project started without a drawing.
app-import-dxf-into-project = Importera DXF eller SVG till projektet…
app-import-dxf-into-project-path = Importera DXF eller SVG till projektet från sökväg…
# File menu, a new version of the project's drawing.
app-update-drawing-from-dxf = Uppdatera ritning från DXF eller SVG…
app-update-drawing-from-path = Uppdatera ritning från sökväg…
