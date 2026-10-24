# The report panel under the canvas: what the last action had to say, and the drawing's
# facts (crates/chipload-gui/src/app/report.rs).

# The panel's name, a noun.
report-heading = Informe
report-hide-hover = Ocultar el informe
report-show-hover = Mostrar el informe
# On the closed panel's bar: warnings below the headline that can't be seen.
report-more-warnings = { $count ->
        [one] { $count } advertencia más
       *[other] { $count } advertencias más
    }
# The heading of the drawing's facts. An issue is something the import repaired or ignored.
report-drawing-title = Dibujo: { $contours ->
        [one] { $contours } contorno
       *[other] { $contours } contornos
    }, { $issues ->
        [0] sin incidencias
        [one] { $issues } incidencia
       *[other] { $issues } incidencias
    }
report-select-operation-hover = Seleccionar la operación { $number }
