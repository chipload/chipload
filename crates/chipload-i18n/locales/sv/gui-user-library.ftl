# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-library = Bibliotek
user-library-use-profile = Använd profil…
user-library-use-tool = Använd biblioteksverktyg…
user-library-save-to-library = Spara i biblioteket
user-library-save-to-library-named = Spara i biblioteket…
user-library-save-to-library-hover = Kopierar dessa värden till ditt bibliotek, för andra projekt
user-library-save-to-linked-profile-hover = Skriv dessa värden till profilen den är länkad till, för andra projekt
user-library-save-to-linked-tool-hover = Skriv dessa värden till biblioteksverktyget det är länkat till, för andra projekt
user-library-save-as-new = Spara som nytt…
user-library-save-as-new-hover = Spara dessa värden i ditt bibliotek under ett annat namn
user-library-update-from-library = Uppdatera från biblioteket
user-library-update-from-library-hover = Kopiera bibliotekets värden till projektet (ångra med Ctrl+Z)
user-library-show-in-library = Visa i biblioteket
user-library-show-in-library-hover = Öppna det under Den här datorn i bläddraren
# $var is the name of an environment variable.
user-library-no-settings-folder-set = Ingen inställningsmapp hittades; sätt { $var }
user-library-no-settings-folder-use = Ingen inställningsmapp hittades; sätt { $var } för att använda ett bibliotek.
user-library-no-settings-folder = Ingen inställningsmapp
user-library-kept-in = Sparas i { $path }

user-library-name = Namn
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = Verktygsnummer
user-library-tool-number-hover = Numret det får i ett projekt där numret är ledigt
user-library-cutting-values = Skärdata
user-library-cutting-values-hover = Fylls i för en ny operation med detta verktyg när det inte finns någon operation ovanför att kopiera från. Ändrar aldrig en befintlig operation.
user-library-feed = Matning
user-library-plunge-feed = Nedmatning
user-library-step-down = Skärdjup
user-library-stepover = Sidosteg
user-library-safe-z = Säker Z
user-library-max-overcut = Max djup under ämnet
user-library-max-overcut-hover = Hur långt en fräsning får gå under ämnet (ned i offerskivan)

user-library-settings-not-saved = Inställningarna sparades inte: { $error }
user-library-recent-gone = { $path } finns inte längre; den togs bort från listan över senaste.
# Followed by the reason on the next line.
user-library-post-named = Postprocessor "{ $name }":
user-library-post-chosen = Postprocessor: { $name }
user-library-no-machine-profile = Det finns ingen maskinprofil "{ $name }".
user-library-no-library-tool = Det finns inget biblioteksverktyg "{ $name }".
user-library-no-project-tool = Projektet har inget verktyg { $id }.
user-library-machine-profile-named = Maskinprofil "{ $name }"
user-library-library-tool-named = Biblioteksverktyg "{ $name }"
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what }: inga värden ändrade
user-library-values-changed-status = { $what }: { $count ->
    [one] { $count } värde ändrat
   *[other] { $count } värden ändrade
}
user-library-changes-heading = { $what }:
user-library-no-values-changed = inga värden ändrade

user-library-profile-gone = Profilen den kom från finns inte i ditt bibliotek.
user-library-library-tool-gone = Biblioteksverktyget det kom från finns inte i ditt bibliotek.
user-library-no-tool-to-save = Projektet har inget verktyg att spara.
# Followed by the reason on the next line.
user-library-not-saved = Sparades inte i biblioteket:
user-library-saved = Sparade "{ $name }" i ditt bibliotek

# Dialog titles.
user-library-save-machine-title = Spara maskinprofil
user-library-save-tool-title = Spara verktyg i biblioteket
user-library-name-in-library = Namn i ditt bibliotek:
user-library-replaces = Ersätter "{ $name }" i ditt bibliotek.
# Buttons.
user-library-replace = Ersätt

user-library-tool-exists = Det finns redan ett verktyg "{ $name }".
user-library-machine-exists = Det finns redan en maskin "{ $name }".
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = Nytt verktyg
user-library-new-machine = Ny maskin
