# Recovery copies of unsaved work, and the window that offers them at start
# (crates/chipload-gui/src/app/recovery.rs).

recovery-copies-off = Återställningskopior är avstängda: { $error }
# Followed by the reason on the next line.
recovery-cannot-open = Kan inte öppna återställningskopian:
recovery-recovered = Återställde osparat arbete: { $title }. Spara det för att behålla det.

recovery-title = Återställ osparat arbete
recovery-intro = Chipload stängdes utan att spara dessa ändringar:
# A copy of a project that was never saved to a file.
recovery-untitled = Namnlöst (aldrig sparat)

# When the unsaved changes in a copy were made.
recovery-changes-unknown = Ändringar från okänd tidpunkt
recovery-changes-moments = Ändringar från mindre än en minut sedan
recovery-changes-minutes = { $count ->
    [one] Ändringar från { $count } minut sedan
   *[other] Ändringar från { $count } minuter sedan
}
recovery-changes-hours = { $count ->
    [one] Ändringar från { $count } timme sedan
   *[other] Ändringar från { $count } timmar sedan
}
recovery-changes-days = { $count ->
    [one] Ändringar från { $count } dag sedan
   *[other] Ändringar från { $count } dagar sedan
}

recovery-saved-since = Projektfilen har sparats sedan denna kopia gjordes.
recovery-cannot-be-opened = Kan inte öppnas: { $problem }

# A button: open the copy.
recovery-open = Öppna
recovery-open-hover = Öppna den som ett osparat projekt; spara för att behålla den
recovery-open-dirty = Spara eller stäng det aktuella projektet först
recovery-open-unreadable = Denna kopia kan inte läsas
# A button: decide about the copy another time.
recovery-later = Senare
recovery-later-hover = Behåll den och fråga igen nästa gång
