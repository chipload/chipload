# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Report
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Disegno: { $contours ->
        [one] { $contours } contorno
       *[other] { $contours } contorni
    }, { $issues ->
        [0] nessuna anomalia
        [one] { $issues } anomalia
       *[other] { $issues } anomalie
    }
report-select-operation-hover = Seleziona lavorazione { $number }
# A line of a list in the report, indented under its heading: $line is a message from
# the project (a problem). Only the two spaces in front are this text's.
report-indented = {"  "}{ $line }
report-select-tool-hover = Apri l'utensile T{ $tool }
report-update-tools-hover = Dai a questi utensili i valori della tua libreria, in un solo passo di annullamento
