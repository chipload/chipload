# The user's library in the Browser and in Properties (crates/chipload-gui/src/app/library_view.rs).

# Buttons in a library folder's menu and properties.
library-view-new-tool = Nuovo utensile
library-view-new-machine = Nuova macchina
library-view-import = Importa…
library-view-export = Esporta…

# The Browser's root for what is kept on this computer, not in the project.
library-view-this-computer = Questo computer
library-view-this-computer-hover = Le tue preferenze, utensili, macchine e post-processori, conservati su questo computer per ogni progetto. Un progetto conserva copie di ciò che usa, così lavora allo stesso modo tra un anno e su un altro computer.
library-view-preferences = Preferenze
library-view-preferences-hover = La scheda di attrezzaggio, l'aggiornamento automatico del percorso utensile, il tema colori
library-view-no-settings-folder = Nessuna cartella impostazioni
# $var is the name of an environment variable.
library-view-no-settings-folder-hover = Imposta { $var } per conservare una libreria
library-view-tools = Utensili
library-view-tools-hover = La tua libreria utensili: trascina un utensile sugli Utensili del progetto per usarlo
library-view-machines = Macchine
library-view-machines-hover = I tuoi profili macchina: i valori con cui iniziano i nuovi progetti
library-view-tool-hover = Clic per modificarlo; trascinalo sugli Utensili del progetto per usarlo lì

# Commands on a library tool or machine profile.
library-view-add-to-project = Aggiungi al progetto
library-view-add-to-project-hover = Copialo nel progetto come utensile a sé
library-view-use-in-project = Usa in questo progetto
library-view-use-in-project-hover = Copia i suoi valori nella macchina del progetto (annulla con Ctrl+Z)
library-view-entry-disabled-hover = Apri un progetto, e prima salva o annulla le modifiche
# A button: make this entry the one new projects start with.
library-view-make-default = Rendi predefinito
library-view-make-default-hover = I nuovi progetti iniziano con questo
library-view-delete-hover = Toglilo dalla tua libreria; i progetti conservano le loro copie
library-view-save-hover = Scrivi le modifiche nella tua libreria
library-view-revert = Ripristina
library-view-revert-hover = Scarta le modifiche
library-view-not-saved = non salvato

library-view-tool-library = Libreria utensili
library-view-tool-library-about = Un progetto conserva una copia di ogni utensile che usa. Fai clic su un utensile sotto Questo computer per modificarlo; trascinalo sugli Utensili del progetto, o fai clic destro su di esso, per usarlo.
library-view-machines-about = Un progetto conserva una copia dei valori della sua macchina. Clic destro su un profilo per usarlo in questo progetto; il predefinito è quello con cui iniziano i nuovi progetti.
library-view-tool-count = { $count ->
    [one] { $count } utensile
   *[other] { $count } utensili
} nella tua libreria.
library-view-machine-count = { $count ->
    [one] { $count } macchina
   *[other] { $count } macchine
} nella tua libreria.

library-view-library-tool = Utensile di libreria
library-view-tool-gone = Questo utensile non è più nella tua libreria.
library-view-machine-profile = Profilo macchina
library-view-profile-gone = Questo profilo non è più nella tua libreria.
library-view-entry-about = Nella tua libreria, per ogni progetto. Le modifiche vengono conservate quando premi Salva.
# $tools lists the project's tools, like "T1, T3".
library-view-project-copies = Questo progetto ne ha una copia: { $tools }
library-view-project-machine = La macchina di questo progetto viene da qui.

library-view-deleted = "{ $name }" eliminato dalla tua libreria; i progetti conservano le loro copie
library-view-saved = Salvato nella tua libreria

# Notes at the right end of a library entry's row in the Browser.
library-view-note-default = predefinito
library-view-note-unsaved = non salvato
