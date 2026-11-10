# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = Användarhandboken följer med varje version som chipload-guide-<version>.zip (öppna index.html). Från källkoden: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Chipload-projekt
dialogs-filter-dxf = Ritningar (DXF, SVG)

dialogs-selector-copied = Selektorn kopierad till Urklipp

dialogs-about-title = Om Chipload
dialogs-about-appimage = Linux-AppImage innehåller även libxkbcommon och libxcb-xkb (MIT-liknande licenser); deras upphovsrättsfiler finns i usr/share/doc/chipload/bundled inuti AppImage-filen.
dialogs-third-party = Tredjepartslicenser

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Ritningens enheter
dialogs-units-unknown = { $file } anger inte vilka enheter den använder.
dialogs-units-question = Vilka enheter ritades den i?
# A button: the drawing was drawn in inches.
dialogs-inch = tum
dialogs-units-cancelled = Ritningen importerades inte: dess enheter är okända.
dialogs-px-title = Pixlar per tum
dialogs-px-unknown = { $file } anger sin storlek bara i pixlar.
dialogs-px-question = Hur många pixlar blir en tum?
dialogs-px-96 = 96 (Inkscape, webbläsare)
dialogs-px-72 = 72 (äldre Illustrator)
dialogs-px-other = Annat:
dialogs-px-use = Använd
dialogs-px-cancelled = Ritningen importerades inte: dess storlek är okänd.
dialogs-export-dxf-title = Exportera ritningen som DXF
dialogs-export-dxf-inch = I tum (annars mm)
dialogs-export-dxf-hidden = Även dolda lager och grupper
dialogs-export-dxf-construction = Även hjälpgeometri (på egna lager)
dialogs-export-dxf-selection = Bara de valda formerna
dialogs-export-dxf-save = Exportera…
dialogs-export-dxf-path = Exportera till sökväg…
dialogs-path-export-dxf = DXF-filen att skriva:
dialogs-filter-dxf-file = DXF-ritning

dialogs-unsaved-title = Osparade ändringar
dialogs-unsaved-body = Projektet har ändringar som inte är sparade. Spara dem först?
dialogs-save-hover = Spara och fortsätt sedan
dialogs-dont-save = Spara inte
dialogs-dont-save-hover = Kasta ändringarna och fortsätt

# Another program saved the open project's file.
dialogs-changed-title = { $file } har ändrats på disken
dialogs-changed-body = Ett annat program har sparat projektet. Läsa in dess version? Dina osparade ändringar finns kvar ett Ctrl+Z bort.
dialogs-load-it = Läs in den
dialogs-load-it-hover = Visa filens version; Ctrl+Z tar tillbaka din
dialogs-keep-mine = Behåll min
dialogs-keep-mine-hover = Behåll dina ändringar; nästa Spara skriver över filen

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Öppna sökväg
dialogs-path-open = Ett .chipload-projekt, eller en .dxf eller .svg:
dialogs-path-save-title = Spara projekt
dialogs-path-save = Spara projektet till:
dialogs-path-export-title = Exportera G-kod
dialogs-path-export = Exportera den kontrollerade G-koden till:
dialogs-path-update-title = Uppdatera ritning
dialogs-path-update = DXF- eller SVG-filen:
dialogs-path-image-title = Importera bild
dialogs-path-image = En bild (PNG, JPEG eller BMP):
dialogs-path-add-dxf-title = Lägg till DXF eller SVG
dialogs-path-add-dxf = DXF-filen som ska läggas till i projektet:
dialogs-path-update-dxf-title = Uppdatera ritningsfil
dialogs-path-update-dxf = Den nya versionen av DXF- eller SVG-filen:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = nytt projekt

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Bryt upp { $name } i former
dialogs-explode-body = Dess former blir ritade former som du kan ändra, flytta och radera. Operationerna fräser samma former som förut. Projektet känner inte längre till DXF-filen: det går inte att uppdatera den från DXF:en.
dialogs-explode-image-body = Dess former blir ritade former som du kan ändra, flytta och radera. Operationerna fräser samma former som förut. Själva bilden försvinner ur projektet: den kan inte spåras om.
dialogs-explode-text-body = Dess bokstäver blir ritade former som du kan ändra, flytta och radera. Operationerna fräser samma former som förut. Orden kan inte redigeras längre.
dialogs-explode-join = Foga ihop ändar närmare än
dialogs-explode = Bryt upp
dialogs-join-title = Foga ihop ändar
dialogs-join-body = Öppna polylinjer bland { $count } { $count ->
    [one] markerad form
   *[other] markerade former
} vars ändar ligger närmare än glappet blir en, på samma lager. En kedja som kommer tillbaka till sin början blir sluten.
dialogs-join-body-all = Öppna ritade polylinjer vars ändar ligger närmare än glappet blir en, på samma lager. En kedja som kommer tillbaka till sin början blir sluten.
dialogs-join-gap = Foga ihop ändar närmare än
dialogs-join = Foga ihop

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Inläggning…
inlay-ribbon-hover = En V-graverad inläggning av de valda formerna: en ficka här, och en spegelvänd plugg i en egen skiva som limmas i den
inlay-menu = Ny inläggning…
inlay-title = V-graverad inläggning
inlay-body = De valda formerna graveras som en ficka i den här skivan, och som en plugg, spegelvänd, i en ny uppspänning: en egen skiva. Pluggen limmas med framsidan ned i fickan, och dess baksida hyvlas bort.
inlay-vbit = V-fräs
inlay-none = Ingen
inlay-depth = Fickans djup
inlay-depth-hover = Hur djupt fickan fräses i grundskivan
inlay-plug-start = Pluggens startdjup
inlay-plug-start-hover = Hur djupt i sin skiva pluggens kontur tas: så djupt ligger dess framsida i fickan, och resten av fickans djup blir kvar för limmet
inlay-gap = Limspalt
inlay-gap-hover = Hur mycket djupare pluggskivan fräses, så att dess baksida står så högt över grundskivan och utpressat lim får plats
inlay-floor = Pinnfräs för botten
inlay-floor-hover = En pinnfräs för bägges bottenpass. Pluggens kanal är mest botten
inlay-thickness = Pluggskivans tjocklek
inlay-thickness-hover = Skivan pluggen fräses ur. Det som blir kvar under kanalen håller ihop den tills det hyvlas bort
inlay-make = Gör inläggning
inlay-no-vbit = Välj en V-fräs: varken projektet eller ditt verktygsbibliotek har någon
inlay-made = Inläggningen är gjord: operation { $pocket } är fickan, operation { $plug } pluggen, i uppspänning { $setup }, dess del spegelvänd vänster–höger.
inlay-refused = Inläggningen kunde inte göras.
sign-title = Ny skylt
sign-body = Orden, en ram runt dem som följer dem, och varje operation som skär skylten, i en egen del. Allt går att ändra efteråt.
sign-words = Ord
sign-words-hint = Vad skylten säger; Enter börjar en ny rad
sign-around = Runt markeringen
sign-around-body = { $count ->
    [one] Skylten går runt den markerade formen, som snidas med V-fräsen. Ramen följer den, även en bild som spåras om.
   *[other] Skylten går runt de { $count } markerade formerna, som snidas med V-fräsen. Ramen följer dem, även en bild som spåras om.
}
sign-around-words = Skriv ord i stället
sign-around-words-hover = Gör skylten runt nya ord, som utan markering
sign-font = Typsnitt
sign-height = Höjd
sign-style = Stil
sign-style-hover = Graverad: bokstäverna V-snidade. Upphöjd: bakgrunden innanför den innersta kanten sänkt runt bokstäverna. Inläggning: bokstäverna inlagda från en andra bräda.
sign-style-engraved = Graverad
sign-style-raised = Upphöjd
sign-style-inlay = Inläggning
sign-relief = Bakgrundens djup
sign-relief-hover = Hur djupt bakgrunden runt de upphöjda bokstäverna fräses, med pinnfräsen
sign-edge = Kant
sign-edge-chamfer = Fas
sign-edge-hover = En så bred fas runt ytterlinjen, skuren med V-fräsen innan skylten skärs ut
sign-vbit = V-fräs
sign-end-mill = Pinnfräs
wizard-library-tool-hover = Från ditt verktygsbibliotek: kopieras in i projektet när du bekräftar, inte förr
wizard-library-tool-gone = Ett verktyg som valdes ur ditt bibliotek finns inte där längre: välj ett annat
two-sided-title = Tvåsidig
two-sided-body = Skivan fräses på ena sidan, vänds och fräses på den andra. Styrpinnar genom två hål på vändaxeln lägger tillbaka den exakt.
two-sided-front = Framsida
two-sided-front-hover = Uppspänningen som fräses först, med framsidan upp; baksidan är samma skiva vänd
two-sided-diameter = Pinndiameter
two-sided-diameter-hover = Styrpinnarnas diameter: hålen skärs i den storleken, genom skivan
two-sided-depth = Ner i offerskivan
two-sided-depth-hover = Hur långt hålen går ner i offerskivan, så att pinnarna står fast
two-sided-tool = Skärs med
two-sided-tool-hover = Ett borr i pinnarnas storlek borrar dem; en pinnfräs fräser ur dem (ett hål minst 1,2 gånger dess diameter)
two-sided-tool-automatic = Valt för hålen
two-sided-zero = Nollpunkt
two-sided-zero-check = Vid första pinnen
two-sided-zero-hover = Framsidans nollpunkt i första pinnens mitt: pinnen hamnar på sig själv när skivan vänds, så båda sidor delar den exakt
two-sided-no-room = Inget utrymme för pinnarna: { $room } bredvid skären på vändaxeln, och ett hål behöver { $needed }.
two-sided-widen = Bredda marginalen till { $margin }
two-sided-widen-hover = Skivan som anpassas runt ritningen växer: den ändras först när du gör baksidan
two-sided-no-room-own-board = Gör skivan längre längs vändaxeln i dess Ämne-formulär, eller pinnarna mindre.
two-sided-board-grows = Skivan blir { $grows } längre åt varje håll: såga den till den nya storleken.
two-sided-make = Gör baksidan
two-sided-change = Ändra baksidan
two-sided-made = Lade till { $back }.
two-sided-made-pins = Lade till { $back }, och operation { $number } skär dess styrhål.
two-sided-changed = Ändrade { $back }.
two-sided-refused = Baksidan kan inte göras
sign-board = Bräda
sign-fit-board = Anpassa runt skylten
sign-fit-board-hover = Ämnet anpassat runt ritningen med den här marginalen, så här tjockt. Av: ämnet är som det är.
sign-thickness = Tjocklek
sign-board-margin = Marginal
sign-adds = Lägger till: { $operations }
sign-make = Gör skylt
sign-no-tools = Välj en V-fräs och en pinnfräs, projektets eller ur ditt verktygsbibliotek.
sign-made = Skylten gjord: { $count } operationer i en egen del. Ändra orden så följer ramen.
sign-refused = Kunde inte göra skylten.
inlay-mark-gap = baksida, { $gap } över
inlay-mark-surface = yta
inlay-mark-face = pluggens framsida, { $depth }
inlay-mark-pocket = ficka, { $depth }
