# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulation: { $volume } cm³ abgetragen, Zellen { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Tiefster Schnitt: Z{ $z } ({ $depth } unter der Rohteiloberkante)
pipeline-simulation-nothing-cut = {"  "}Nichts gefräst
pipeline-simulation-below-bottom = {"  "}Fräst { $depth } unter die Rohteilunterkante (in die Opferplatte)
pipeline-simulation-drill-point = {"  "}Bohrer mit einer 118°-Spitze simuliert
pipeline-simulation-vbit-angle = {"  "}Ein V-Fräser ohne Winkel als 90° simuliert
