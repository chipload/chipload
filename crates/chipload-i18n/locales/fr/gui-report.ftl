# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Rapport
report-hide-hover = Masquer le rapport
report-show-hover = Afficher le rapport
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } autre avertissement
       *[other] { $count } autres avertissements
    }
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Dessin : { $contours ->
        [one] { $contours } contour
       *[other] { $contours } contours
    }, { $issues ->
        [0] aucune anomalie
        [one] { $issues } anomalie
       *[other] { $issues } anomalies
    }
report-select-operation-hover = Sélectionner l'opération { $number }
# A line of a list in the report, indented under its heading: $line is a message from
# the project (a problem). Only the two spaces in front are this text's.
report-indented = {"  "}{ $line }
