# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulazione: { $volume } cm³ rimossi, celle { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Taglio più profondo: Z{ $z } ({ $depth } sotto la superficie del grezzo)
pipeline-simulation-nothing-cut = {"  "}Nessun taglio
pipeline-simulation-below-bottom = {"  "}Taglia { $depth } sotto il fondo del grezzo (dentro il piano di sacrificio)
pipeline-simulation-drill-point = {"  "}Punte simulate con vertice a 118°
pipeline-simulation-vbit-angle = {"  "}Fresa a V senza angolo simulata a 90°
