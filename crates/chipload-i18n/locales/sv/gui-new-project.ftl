# The new project dialog (crates/chipload-gui/src/app/new_project.rs, docs/specs/new-project-dialog.md).

new-project-title = Nytt projekt
new-project-material = Material
new-project-material-none = Inget
new-project-thickness = Tjocklek
# The board: fitted round the drawing with a margin, or a size.
new-project-board = Skiva
new-project-fit = Anpassad till ritningen, marginal
new-project-size = Storlek
new-project-z-zero = Z-nollpunkt
new-project-z-zero-hover = Var maskinens Z0 sätts: på skivans ovansida eller på offerskivan under den
new-project-z-top = Skivans ovansida
new-project-z-spoilboard = Offerskivan
new-project-machine = Maskin
new-project-machine-none = Chiploads egna inställningar (ingen maskin i biblioteket)
# Under the machine: its post-processor, G-code units (mm or inch) and how tools are changed.
new-project-machine-summary = { $post } · { $units } · { $change }
new-project-tools = Verktyg
new-project-search = Sök
# Under the tool list: how many are ticked and their numbers ("T1, T2").
new-project-tools-count = { $count ->
    [0] Inga verktyg: lägg till ett innan du genererar
    [one] 1 verktyg: { $list }
   *[other] { $count } verktyg: { $list }
}
new-project-no-library-tools = Verktygsbiblioteket är tomt: projektet börjar med en 6 mm pinnfräs. Lägg till dina verktyg under Den här datorn > Verktyg.
new-project-create = Skapa
