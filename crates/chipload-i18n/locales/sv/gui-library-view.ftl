# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = Nytt verktyg
library-view-new-machine = Ny maskin
library-view-import = Importera…
library-view-export = Exportera…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = Den här datorn
library-view-this-computer-hover = Dina inställningar, verktyg, maskiner och postprocessorer, sparade på den här datorn för alla projekt. Ett projekt behåller kopior av det det använder, så det fräses likadant om ett år och på en annan dator.
library-view-preferences = Inställningar
library-view-preferences-hover = Uppsättningsbladet, automatisk uppdatering av verktygsbanan, färgtemat
library-view-no-settings-folder = Ingen inställningsmapp
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Sätt { $var } för att spara ett bibliotek
library-view-tools = Verktyg
library-view-tools-hover = Ditt verktygsbibliotek: dra ett verktyg till projektets Verktyg för att använda det
library-view-machines = Maskiner
library-view-machines-hover = Dina maskinprofiler: värdena som nya projekt startar med
library-view-tool-hover = Klicka för att redigera det; dra det till projektets Verktyg för att använda det där

# Commands on a library tool or machine profile.
library-view-add-to-project = Lägg till i projektet
library-view-add-to-project-hover = Kopiera det till projektet som ett eget verktyg
library-view-use-in-project = Använd i detta projekt
library-view-use-in-project-hover = Kopiera dess värden till projektets maskin (ångra med Ctrl+Z)
library-view-entry-disabled-hover = Öppna ett projekt, och spara eller återställ ändringarna först
# A button: make this entry the one new projects start with.
library-view-make-default = Gör till standard
library-view-make-default-hover = Nya projekt startar med den
library-view-delete-hover = Ta bort den från ditt bibliotek; projekt behåller sina kopior
library-view-save-hover = Skriv ändringarna till ditt bibliotek
library-view-revert = Återställ
library-view-revert-hover = Kasta ändringarna
library-view-not-saved = inte sparat

library-view-tool-library = Verktygsbibliotek
library-view-tool-library-about = Ett projekt behåller en kopia av varje verktyg det använder. Klicka på ett verktyg under Den här datorn för att redigera det; dra det till projektets Verktyg, eller högerklicka på det, för att använda det.
library-view-machines-about = Ett projekt behåller en kopia av maskinens värden. Högerklicka på en profil för att använda den i detta projekt; standardprofilen är den som nya projekt startar med.
library-view-tool-count = { $count ->
    [one] { $count } verktyg
   *[other] { $count } verktyg
} i ditt bibliotek.
library-view-machine-count = { $count ->
    [one] { $count } maskin
   *[other] { $count } maskiner
} i ditt bibliotek.

library-view-library-tool = Biblioteksverktyg
library-view-tool-gone = Detta verktyg finns inte längre i ditt bibliotek.
library-view-machine-profile = Maskinprofil
library-view-profile-gone = Denna profil finns inte längre i ditt bibliotek.
library-view-entry-about = I ditt bibliotek, för alla projekt. Ändringarna sparas när du trycker på Spara.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = Detta projekt har en kopia: { $tools }
library-view-project-machine = Detta projekts maskin kom från den.

library-view-deleted = Raderade "{ $name }" från ditt bibliotek; projekt behåller sina kopior
library-view-saved = Sparat i ditt bibliotek

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = standard
library-view-note-unsaved = osparad
