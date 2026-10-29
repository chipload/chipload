# The library rows of a project tool and the machine, and copying values into and out of the project (crates/chipload-gui/src/app/user_library.rs).

# The user's library of tools and machine profiles, kept on this computer.
user-library-library = Libreria
user-library-use-profile = Usa profilo…
user-library-use-tool = Usa utensile di libreria…
user-library-save-to-library = Salva in libreria
user-library-save-to-library-named = Salva in libreria…
user-library-save-to-library-hover = Copia questi valori nella tua libreria, per altri progetti
user-library-save-to-linked-profile-hover = Scrivi questi valori nel profilo a cui è collegata, per altri progetti
user-library-save-to-linked-tool-hover = Scrivi questi valori nell'utensile di libreria a cui è collegato, per altri progetti
user-library-save-as-new = Salva come nuovo…
user-library-save-as-new-hover = Conserva questi valori nella tua libreria con un altro nome
user-library-update-from-library = Aggiorna da libreria
user-library-update-from-library-hover = Copia i valori della libreria nel progetto (annulla con Ctrl+Z)
user-library-show-in-library = Mostra in libreria
user-library-show-in-library-hover = Aprilo sotto Questo computer nel Browser
# $var is the name of an environment variable.
user-library-no-settings-folder-set = Nessuna cartella impostazioni trovata; imposta { $var }
user-library-no-settings-folder-use = Nessuna cartella impostazioni trovata; imposta { $var } per usare una libreria.
user-library-no-settings-folder = Nessuna cartella impostazioni
user-library-kept-in = Conservata in { $path }

user-library-name = Nome
# The tool's number (T1, T2, …) in a project.
user-library-tool-number = Numero utensile
user-library-tool-number-hover = Il numero che riceve in un progetto dove quel numero è libero
user-library-cutting-values = Valori di taglio
user-library-cutting-values-hover = Inseriti in una nuova lavorazione con questo utensile quando non c'è una lavorazione sopra da cui copiarli. Non modifica mai una lavorazione esistente.
user-library-feed = Avanzamento
user-library-plunge-feed = Affondamento
user-library-step-down = Prof. di passata
user-library-stepover = Passo laterale
user-library-safe-z = Z sicura
user-library-max-overcut = Prof. max sotto grezzo
user-library-max-overcut-hover = Quanto un taglio può scendere sotto il grezzo (nel piano di sacrificio)

user-library-settings-not-saved = Impostazioni non salvate: { $error }
user-library-recent-gone = { $path } non esiste più; è stato tolto dall'elenco dei recenti.
# Followed by the reason on the next line.
user-library-post-named = Post-processore "{ $name }":
user-library-post-chosen = Post-processore: { $name }
user-library-no-machine-profile = Non esiste un profilo macchina "{ $name }".
user-library-no-library-tool = Non esiste un utensile di libreria "{ $name }".
user-library-no-project-tool = Il progetto non ha l'utensile { $id }.
user-library-machine-profile-named = Profilo macchina "{ $name }"
user-library-library-tool-named = Utensile di libreria "{ $name }"
# $what is user-library-machine-profile-named or user-library-library-tool-named.
user-library-no-values-changed-status = { $what }: nessun valore cambiato
user-library-values-changed-status = { $what }: { $count ->
        [one] { $count } valore cambiato
       *[other] { $count } valori cambiati
    }
user-library-changes-heading = { $what }:
user-library-no-values-changed = nessun valore cambiato

user-library-profile-gone = Il profilo da cui proviene non è nella tua libreria.
user-library-library-tool-gone = L'utensile di libreria da cui proviene non è nella tua libreria.
user-library-no-tool-to-save = Il progetto non ha utensili da salvare.
# Followed by the reason on the next line.
user-library-not-saved = Non salvato nella libreria:
user-library-saved = "{ $name }" salvato nella tua libreria

# Dialog titles.
user-library-save-machine-title = Salva profilo macchina
user-library-save-tool-title = Salva utensile in libreria
user-library-name-in-library = Nome nella tua libreria:
user-library-replaces = Sostituisce "{ $name }" nella tua libreria.
# Buttons.
user-library-replace = Sostituisci

user-library-tool-exists = Esiste già un utensile "{ $name }".
user-library-machine-exists = Esiste già una macchina "{ $name }".
# The names a new library entry gets ("New tool 2" if taken).
user-library-new-tool = Nuovo utensile
user-library-new-machine = Nuova macchina
