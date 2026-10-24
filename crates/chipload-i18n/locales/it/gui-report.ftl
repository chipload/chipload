# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Report
report-hide-hover = Nascondi il report
report-show-hover = Mostra il report
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } altro avviso
       *[other] altri { $count } avvisi
    }
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
