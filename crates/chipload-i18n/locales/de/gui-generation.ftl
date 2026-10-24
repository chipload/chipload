# Generating the toolpath and G-code, and the live preview
# (crates/chipload-gui/src/app/generation.rs).

generation-no-project = Importieren Sie zuerst eine DXF oder öffnen Sie ein Projekt.
generation-could-not-start = Berechnung konnte nicht starten: { $error }
generation-cancelled = Berechnung abgebrochen
# What went wrong when the planning stopped without an answer.
generation-worker-stopped = der Hintergrundprozess hat ohne Ergebnis aufgehört

generation-check-failed = G-Code-Prüfung FEHLGESCHLAGEN. Es kann nichts exportiert werden.
# Followed by the reason on the next line.
generation-failed = Berechnung fehlgeschlagen:
generation-failed-unexpectedly = Berechnung unerwartet fehlgeschlagen: { $error }
generation-please-report = Ihr Projekt ist unverändert. Bitte melden Sie dies mit der Projektdatei.

# $post: the post-processor's name, e.g. GRBL.
generation-done = { $lines } Zeilen für { $post } berechnet. Prüfung bestanden.
generation-done-files = { $lines } Zeilen in { $files } Dateien berechnet, eine je Werkzeuggruppe, für { $post }. Prüfung bestanden.
# A warning from the G-code checker.
generation-check-warning = Prüfung: { $message }
# Where X, Y and Z zero are on the stock.
generation-work-zero = Werkstücknullpunkt: { $zero }
# "All parts" is a switch on the bar above the canvas.
generation-one-part-shown = Gezeigt: nur Teil „{ $part }“ (Alle Teile in der Leiste der Zeichenfläche zeigt jedes Teil). Exportieren schreibt jedes gefräste Teil.

generation-export-when-done = Export, sobald der Werkzeugweg berechnet ist
