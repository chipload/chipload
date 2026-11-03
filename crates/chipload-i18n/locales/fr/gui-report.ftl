# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Rapport
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
report-select-tool-hover = Ouvrir l'outil T{ $tool }
report-update-tools-hover = Donner à ces outils les valeurs de votre bibliothèque, en une seule annulation
