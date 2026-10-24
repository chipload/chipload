# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Wiederherstellungskopien sind aus: { $error }
# Followed by the reason on the next line.
recovery-cannot-open = Die Wiederherstellungskopie kann nicht geöffnet werden:
recovery-recovered = Ungespeicherte Arbeit wiederhergestellt: { $title }. Speichern Sie sie, um sie zu behalten.

recovery-title = Ungespeicherte Arbeit wiederherstellen
recovery-intro = Chipload wurde geschlossen, ohne diese Änderungen zu speichern:
# A copy of a project that was never saved to a file.
recovery-untitled = Unbenannt (nie gespeichert)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Änderungen von unbekanntem Zeitpunkt
recovery-changes-moments = Änderungen von vor weniger als einer Minute
recovery-changes-minutes = { $count ->
    [one] Änderungen von vor { $count } Minute
   *[other] Änderungen von vor { $count } Minuten
}
recovery-changes-hours = { $count ->
    [one] Änderungen von vor { $count } Stunde
   *[other] Änderungen von vor { $count } Stunden
}
recovery-changes-days = { $count ->
    [one] Änderungen von vor { $count } Tag
   *[other] Änderungen von vor { $count } Tagen
}

recovery-saved-since = Die Projektdatei wurde gespeichert, nachdem diese Kopie entstand.
recovery-cannot-be-opened = Kann nicht geöffnet werden: { $problem }

# A button: open the copy.
recovery-open = Öffnen
recovery-open-hover = Als ungespeichertes Projekt öffnen; speichern, um es zu behalten
recovery-open-dirty = Speichern oder schließen Sie zuerst das aktuelle Projekt
recovery-open-unreadable = Diese Kopie kann nicht gelesen werden
# A button: decide about the copy another time.
recovery-later = Später
recovery-later-hover = Behalten und beim nächsten Mal erneut fragen
