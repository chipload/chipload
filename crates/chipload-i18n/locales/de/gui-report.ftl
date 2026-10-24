# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Bericht
report-hide-hover = Den Bericht ausblenden
report-show-hover = Den Bericht zeigen
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } weitere Warnung
       *[other] { $count } weitere Warnungen
    }
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
