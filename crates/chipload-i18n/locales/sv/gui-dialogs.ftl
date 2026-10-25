# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = Användarhandboken följer med varje version som chipload-guide-<version>.zip (öppna index.html). Från källkoden: packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Chipload-projekt
dialogs-filter-dxf = DXF-ritningar

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
dialogs-path-open = Ett .chipload-projekt eller en .dxf:
dialogs-path-save-title = Spara projekt
dialogs-path-save = Spara projektet till:
dialogs-path-export-title = Exportera G-kod
dialogs-path-export = Exportera den kontrollerade G-koden till:
dialogs-path-update-title = Uppdatera ritning
dialogs-path-update = DXF-filen:
dialogs-path-image-title = Importera bild
dialogs-path-image = En bild (PNG, JPEG eller BMP):
dialogs-path-add-dxf-title = Lägg till DXF
dialogs-path-add-dxf = DXF-filen som ska läggas till i projektet:
dialogs-path-update-dxf-title = Uppdatera DXF
dialogs-path-update-dxf = Den nya versionen av DXF-filen:

# In the window's title bar, for a project never saved.
dialogs-title-new-project = nytt projekt

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Bryt upp { $name } i former
dialogs-explode-body = Dess former blir ritade former som du kan ändra, flytta och radera. Operationerna fräser samma former som förut. Projektet känner inte längre till DXF-filen: det går inte att uppdatera den från DXF:en.
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
