# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Le copie di recupero sono disattivate: { $error }
# Followed by the reason on the next line.
recovery-cannot-open = Impossibile aprire la copia di recupero:
recovery-recovered = Recuperato lavoro non salvato: { $title }. Salvalo per conservarlo.

recovery-title = Recupera lavoro non salvato
recovery-intro = Chipload si è chiuso senza salvare queste modifiche:
# A copy of a project that was never saved to a file.
recovery-untitled = Senza titolo (mai salvato)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Modifiche di un momento sconosciuto
recovery-changes-moments = Modifiche di meno di un minuto fa
recovery-changes-minutes = { $count ->
    [one] Modifiche di { $count } minuto fa
   *[other] Modifiche di { $count } minuti fa
}
recovery-changes-hours = { $count ->
    [one] Modifiche di { $count } ora fa
   *[other] Modifiche di { $count } ore fa
}
recovery-changes-days = { $count ->
    [one] Modifiche di { $count } giorno fa
   *[other] Modifiche di { $count } giorni fa
}

recovery-saved-since = Il file del progetto è stato salvato dopo che questa copia è stata fatta.
recovery-cannot-be-opened = Non si può aprire: { $problem }

# A button: open the copy.
recovery-open = Apri
recovery-open-hover = Aprila come progetto non salvato; salva per conservarla
recovery-open-dirty = Salva o chiudi prima il progetto corrente
recovery-open-unreadable = Questa copia non si può leggere
# A button: decide about the copy another time.
recovery-later = Più tardi
recovery-later-hover = Conservala e chiedi di nuovo la prossima volta
