# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulation: { $volume } cm³ removed, cells { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Deepest cut: Z{ $z } ({ $depth } below the stock top)
pipeline-simulation-nothing-cut = {"  "}Nothing cut
pipeline-simulation-below-bottom = {"  "}Cuts { $depth } below the stock bottom (into the spoilboard)
pipeline-simulation-drill-point = {"  "}Drills simulated with a 118° point
pipeline-simulation-vbit-angle = {"  "}A V-bit without an angle simulated as 90°
