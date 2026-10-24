# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulación: { $volume } cm³ retirados, celdas de { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Corte más profundo: Z{ $z } ({ $depth } por debajo de la cara superior del material en bruto)
pipeline-simulation-nothing-cut = {"  "}No se ha cortado nada
pipeline-simulation-below-bottom = {"  "}Corta { $depth } por debajo de la cara inferior del material en bruto (dentro del tablero de sacrificio)
pipeline-simulation-drill-point = {"  "}Brocas simuladas con una punta de 118°
pipeline-simulation-vbit-angle = {"  "}Una fresa en V sin ángulo se simula como de 90°
