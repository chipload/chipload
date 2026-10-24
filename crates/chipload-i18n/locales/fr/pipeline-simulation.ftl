# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulation : { $volume } cm³ enlevés, cellules { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Coupe la plus profonde : Z{ $z } ({ $depth } sous le dessus du brut)
pipeline-simulation-nothing-cut = {"  "}Rien n'est usiné
pipeline-simulation-below-bottom = {"  "}Usine { $depth } sous le dessous du brut (dans le martyr)
pipeline-simulation-drill-point = {"  "}Forets simulés avec une pointe à 118°
pipeline-simulation-vbit-angle = {"  "}Une fraise à graver en V sans angle simulée à 90°
