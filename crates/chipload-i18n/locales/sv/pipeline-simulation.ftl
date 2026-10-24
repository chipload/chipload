# The material-removal simulation's lines in the report (crates/chipload-pipeline,
# docs/specs/simulation.md §5). A length argument brings its own unit ("0.5 mm").
# The lines under the first start with spaces, written {"  "}: keep them, they indent the line.

# $volume: cubic centimetres removed; $cell: a cell's size; $columns × $rows cells.
pipeline-simulation-removed = Simulering: { $volume } cm³ borttaget, celler { $cell } ({ $columns } × { $rows })
# $z: the lowest height cut; $depth: how far that is below the stock top.
pipeline-simulation-deepest = {"  "}Djupaste fräsning: Z{ $z } ({ $depth } under ämnets ovansida)
pipeline-simulation-nothing-cut = {"  "}Inget fräst
pipeline-simulation-below-bottom = {"  "}Fräser { $depth } under ämnets undersida (ned i offerskivan)
pipeline-simulation-drill-point = {"  "}Borrar simulerade med 118° spets
pipeline-simulation-vbit-angle = {"  "}En V-fräs utan vinkel simulerad som 90°
