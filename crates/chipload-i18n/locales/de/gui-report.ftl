# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Bericht
report-details = Details ({ $count })
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Zeichnung: { $contours ->
        [one] { $contours } Kontur
       *[other] { $contours } Konturen
    }, { $issues ->
        [0] keine Auffälligkeiten
        [one] { $issues } Auffälligkeit
       *[other] { $issues } Auffälligkeiten
    }
report-select-operation-hover = Bearbeitung { $number } auswählen
# A line of a list in the report, indented under its heading: $line is a message from
# the project (a problem). Only the two spaces in front are this text's.
report-indented = {"  "}{ $line }
report-select-tool-hover = Werkzeug T{ $tool } öffnen
report-update-tools-hover = Diesen Werkzeugen die Werte Ihrer Bibliothek geben, als ein Rückgängig-Schritt
report-place-dowels = Passstiftbohrungen neu platzieren
report-place-dowels-hover = Die zwei Kreise dorthin verschieben, wo sie jetzt hingehören, frei von den Schnitten beider Seiten, als ein Rückgängig-Schritt. Nicht, wenn die Vorderseite schon bearbeitet ist: ihre Löcher sind gebohrt
report-part = Teil { $number } · { $name }
report-file = Datei { $number }
