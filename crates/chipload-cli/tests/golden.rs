// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden G-code: `chipload run` on the frozen corpus projects (tests/corpus/jobs/v4 on)
//! must reproduce tests/corpus/expected/ byte for byte, twice in a row. The same projects
//! without arcs must reproduce tests/corpus/expected/lines/ (docs/specs/arc-output.md).
//!
//! When a change is intended, regenerate with `CHIPLOAD_UPDATE_GOLDEN=1 cargo test -p
//! chipload-cli --test golden`, review the diff, and explain it in the commit message.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chipload_core::math;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

/// (golden name, frozen project). The names say which schema version the job first had;
/// the projects are frozen at the version in their path and migrate on load.
const GOLDEN: [(&str, &str); 90] = [
    ("v1_example_job", "jobs/v4/example_job.chipload"),
    ("v2_simple_part_full", "jobs/v4/simple_part_full.chipload"),
    ("v3_messy_2_lower", "jobs/v4/messy_2_lower.chipload"),
    ("v3_simple_part_tabs", "jobs/v4/simple_part_tabs.chipload"),
    (
        "v3_messy_2_lower_tabs",
        "jobs/v4/messy_2_lower_tabs.chipload",
    ),
    ("v5_drawn_part", "jobs/v5/drawn_part.chipload"),
    // The same project frozen at v6: must give the same G-code after migrating to v7.
    ("v6_drawn_part", "jobs/v6/drawn_part.chipload"),
    // Start depth, clearance (both signs) and a cut into the spoilboard.
    (
        "v7_depths_and_clearance",
        "jobs/v7/depths_and_clearance.chipload",
    ),
    ("v7_ramp_entry", "jobs/v7/ramp_entry.chipload"),
    // The ramp-entry part cut six times, operation by operation (docs/specs/copies-grid.md).
    ("v7_copies", "jobs/v7/copies.chipload"),
    // The ramp-entry part with leads and moved starts (docs/specs/helix-and-leads.md).
    ("v7_leads", "jobs/v7/leads.chipload"),
    // The same, its pockets going down round a helix (helical G2/G3).
    ("v7_helix", "jobs/v7/helix.chipload"),
    // The simple part frozen at v7 (an imported drawing): the same G-code as
    // v2_simple_part_full after migrating to v8 (docs/specs/drawing-placement.md).
    ("v7_simple_part_full", "jobs/v7/simple_part_full.chipload"),
    // The same, its drawing moved by (-20, 30) with a start point on the outline.
    ("v8_placed_part", "jobs/v8/placed_part.chipload"),
    // The same, turned 90° and put in the stock's corner (docs/specs/drawing-rotation.md).
    ("v9_turned_part", "jobs/v9/turned_part.chipload"),
    // Serif letters V-carved with a 60° bit in two passes (docs/specs/v-carve.md).
    ("v10_vcarve_letters", "jobs/v10/vcarve_letters.chipload"),
    ("v11_scaled_part", "jobs/v11/scaled_part.chipload"),
    // A pocket with a 3 mm tool, the outline with 6 mm: one file per tool
    // (docs/specs/tool-per-operation.md), v11_two_tools-1-T2.nc and v11_two_tools-2-T1.nc.
    ("v11_two_tools", "jobs/v11/two_tools.chipload"),
    // The same in one file, T M6 at the tool change (machine tool_change: program).
    (
        "v12_two_tools_program",
        "jobs/v12/two_tools_program.chipload",
    ),
    // v7_helix frozen at v13, a rings pocket: v14 keeps its corners sharp, the same G-code
    // as v7_helix (docs/specs/pocket-round-corners.md).
    ("v13_helix_rings", "jobs/v13/helix_rings.chipload"),
    // A rings pocket round two islands with rounded ring corners
    // (docs/specs/pocket-round-corners.md).
    ("v14_round_corners", "jobs/v14/round_corners.chipload"),
    // v7_helix (helix, leads, tabs, arcs) with the LinuxCNC and Mach3/4 posts
    // (docs/specs/user-posts.md §5): the same moves in their dialects.
    ("v15_linuxcnc_helix", "jobs/v15/linuxcnc_helix.chipload"),
    ("v15_mach3_helix", "jobs/v15/mach3_helix.chipload"),
    ("v15_straight_tabs", "jobs/v15/straight_tabs.chipload"),
    ("v16_profile_spiral", "jobs/v16/profile_spiral.chipload"),
    ("v17_follow_path", "jobs/v17/follow_path.chipload"),
    // Finishing laps and an onion skin, with ramped tabs (docs/specs/finish-and-skin.md).
    ("v17_finish_skin", "jobs/v17/finish_skin.chipload"),
    // Drilled holes in pecks and helical bores, one through with a core (docs/specs/holes.md).
    ("v18_holes", "jobs/v18/holes.chipload"),
    // Dogbones in a mortise pocket and a notched outline (with tabs), T-bones in a slot
    // (docs/specs/dogbones.md).
    ("v18_dogbones", "jobs/v18/dogbones.chipload"),
    // A 12 mm pocket round an island with a slot too narrow for it, then a 4 mm rest pocket
    // (docs/specs/rest-machining.md).
    ("v18_rest", "jobs/v18/rest.chipload"),
    // A picture traced into shapes and V-carved (docs/specs/image-trace.md §9).
    ("v19_image_vcarve", "jobs/v19/image_vcarve.chipload"),
    // A picture of strokes traced into centre lines with arcs and followed with a 3 mm end
    // mill (docs/specs/image-arcs-centrelines.md §8).
    (
        "v20_image_centre_line",
        "jobs/v20/image_centre_line.chipload",
    ),
    // v11_scaled_part frozen at v20: v21 makes its DXF the first of a list; the same G-code
    // as v11_scaled_part (docs/specs/drawing-sources.md §8).
    ("v20_scaled_part", "jobs/v20/scaled_part.chipload"),
    // Two DXFs (the second added, scaled and moved), a profile of a layer both have, an
    // outline limited to each, and a traced logo V-carved (docs/specs/drawing-sources.md §8).
    ("v21_two_dxfs", "jobs/v21/two_dxfs.chipload"),
    // v7_copies cut copy by copy, frozen by Chipload 0.16.0 with `order: by_part` and its
    // G-code from that binary: v22 renames the order `by_copy` and moves the copies into
    // the one part; the G-code stays the same (docs/specs/parts.md §4).
    ("v21_copies_by_part", "jobs/v21/copies_by_part.chipload"),
    // Two parts (docs/specs/parts.md): DXF 1 with its holes, outline and a V-carved logo,
    // then DXF 2's outline in a 1 × 2 grid. Part by part: T1, T2, then T1 again, and a
    // comment where each part starts.
    ("v22_two_parts", "jobs/v22/two_parts.chipload"),
    // v22_two_parts with part B's DXF turned 90° too (both DXFs quarter-turned) and its
    // copies side by side, frozen at v23 with G-code from that code: v24 makes the quarter
    // turns an angle in degrees; the G-code stays the same (docs/specs/direct-placement.md).
    ("v23_turned_parts", "jobs/v23/turned_parts.chipload"),
    // v14_round_corners (a turned rectangle, pocketed and cut round) frozen at v24: v25
    // gives rectangles a corner radius, 0 for these; the G-code stays the same
    // (docs/specs/rectangle-corner-radius.md).
    ("v24_rectangle_part", "jobs/v24/rectangle_part.chipload"),
    // The same with the rectangle's corners rounded at 15 mm: rings and the outline follow
    // the corner arcs (docs/specs/rectangle-corner-radius.md).
    (
        "v25_rounded_rectangle",
        "jobs/v25/rounded_rectangle.chipload",
    ),
    // v15_straight_tabs frozen at v26: a profile that plunged, with straight tabs and a
    // lead; since v27 every profile spirals down, at 3° here, and a profile has no lead
    // (docs/specs/profile-always-spiral.md).
    ("v26_straight_tabs", "jobs/v26/straight_tabs.chipload"),
    // Text (docs/specs/text.md): two centred lines in FreeSans Bold, turned 8°, V-carved,
    // and a FreeSerif A pocketed round its counter.
    ("v27_text", "jobs/v27/text.chipload"),
    // Pockets that used the removed trochoidal and adaptive clearing (ADR 0017): since v26
    // they are rings pockets like any other (docs/specs/pocket.md as built 2026-10-15).
    ("v7_trochoidal", "jobs/v7/trochoidal.chipload"),
    ("v7_adaptive", "jobs/v7/adaptive.chipload"),
    ("v12_adaptive", "jobs/v12/adaptive.chipload"),
    (
        "v13_adaptive_two_islands",
        "jobs/v13/adaptive_two_islands.chipload",
    ),
    // Two pockets with a 12 mm tool, each with a 4 mm rest pass, then the outline: the
    // passes wait for both pockets, and the outline comes last (docs/specs/rest-passes.md
    // §3): files T1 (1, 2), T2 (1.1, 2.1), T1 (3).
    ("v28_rest_passes", "jobs/v28/rest_passes.chipload"),
    // A surface of the whole stock with a 25 mm endmill, first in part 1, before two parts
    // (part B with two copies): two depths back and forth, the second coming back the
    // other way, and the stock faced once (docs/specs/surfacing.md).
    ("v29_surface", "jobs/v29/surface.chipload"),
    // A part drawn and edited only with the editing commands (docs/specs/drawing-editing.md
    // §9): a pentagon pocketed, its sharp offset filleted and scalloped by a polar array
    // of circles subtracted, cut round. `cli.rs` draws it again from the commands.
    ("v30_drawn_edited", "jobs/v30/drawn_edited.chipload"),
    // An SVG (tests/corpus/svg/badge_layers.svg, Inkscape layers in mm, Bézier curves
    // fitted with arcs): layer Pocket pocketed round its island, layer Outline cut round
    // (docs/specs/svg-import.md §11). `tests/svg.rs` in the pipeline imports it again.
    ("v31_svg_badge", "jobs/v31/svg_badge.chipload"),
    // A v33 project (V-carve walls and a floor pass) moved into a setup by migration 34
    // (docs/specs/setups.md §6): the G-code is the one v33 wrote, but for the V-carve's
    // passes, of equal load since schema 35 (docs/specs/v-carve.md decision 12).
    ("v33_vcarve_floor", "jobs/v33/vcarve_floor.chipload"),
    // Two setups (docs/specs/setups.md): 12 mm sides and a 6 mm bottom with its own zero,
    // each a program of its own (`-S1…`, `-S2…`).
    ("v34_two_setups", "jobs/v34/two_setups.chipload"),
    // docs/specs/v-carve.md decision 12: a step down of 1 mm, 3.5 deep, becomes 4 passes of
    // equal load.
    ("v34_vcarve_step_down", "jobs/v34/vcarve_step_down.chipload"),
    // Built-in fonts beyond FreeFont (docs/specs/more-fonts.md): two lines of joined
    // Satisfy script, merged where the letters overlap, V-carved; an Ultra B pocketed.
    ("v29_script_text", "jobs/v29/script_text.chipload"),
    // v25_rounded_rectangle with a hexagon (a drawn polygon, schema 30) in place of the
    // triangle: rings round the circle and the hexagon, then the rounded outline
    // (docs/specs/drawing-input.md §5).
    ("v30_polygon", "jobs/v30/polygon.chipload"),
    // A sign (docs/specs/shape-library.md §9): a text framed with `frame add` (rounded, one
    // border), the letters V-carved, the border grooved with the V-bit along it, the
    // outline cut out. `cli.rs` draws it again from the commands.
    ("v35_sign_frame", "jobs/v35/sign_frame.chipload"),
    // v34_vcarve_step_down with an ordinary step down of 1 mm (docs/specs/
    // vcarve-step-down.md): 3.5 mm in four equal passes of 0.875 mm, not of equal load.
    ("v35_vcarve_step_down", "jobs/v35/vcarve_step_down.chipload"),
    // simple_part made with `op add` alone (docs/specs/styles.md §2): a pocket of the
    // circle 3 mm deep, then the whole DXF as two profiles, the holes inside, then the
    // outline outside with tabs, both through. `cli.rs` makes it again from the commands.
    ("v35_op_add", "jobs/v35/op_add.chipload"),
    // simple_part made with `style use` alone (docs/specs/styles.md §3): the styles of
    // tests/corpus/styles/v1, a pocket of the circle and a cut out, which on the whole DXF
    // is the holes inside, then the outline outside with tabs. `cli.rs` makes it again.
    ("v35_style_use", "jobs/v35/style_use.chipload"),
    // simple_part's outline cut out with its 4 tabs along X (docs/specs/tabs-along-grain.md):
    // two on the bottom edge and two on the top, at a quarter and three quarters across.
    ("v35_tabs_along", "jobs/v35/tabs_along.chipload"),
    // Two-sided (docs/specs/two-sided.md): v35_op_add's front, surfaced 0.5 mm first, its
    // pocket and holes; then the back, turned left to right, the slot pocketed 4 mm from
    // the back face and the outline cut out from there through the 9.5 mm the surfacing
    // left. The back's program is planned with the cutting sense turned and mirrored about
    // the board's centre line (X 150), so climb stays climb on the back face. Made with
    // `op add-surface`, `setup add --back-of 1`, `op add pocket --shapes 3`, `op part`.
    ("v36_two_sided", "jobs/v36/two_sided.chipload"),
    // A mirrored part (docs/specs/mirror-part.md): v35_op_add's part cut mirrored left to
    // right about its box's centre line, planned with the cutting sense turned so the
    // pocket stays conventional and the profiles climb. Made with `part set --mirror`.
    ("v36_mirrored_part", "jobs/v36/mirrored_part.chipload"),
    // v14_round_corners' pocket (two islands, leads, a moved start) with dogbones and a
    // finishing pass: walls 0.3 mm, floor 0.2 mm (docs/specs/pocket-finish.md).
    ("v36_pocket_finish", "jobs/v36/pocket_finish.chipload"),
    // Chamfers (docs/specs/chamfer.md) with a 90° V-bit with a 0.2 mm flat: 1 mm inside a
    // round and a square hole in one pass, 2 mm round a 120 × 80 box side in two ramped
    // passes; then the holes and the outline cut with an end mill.
    ("v36_chamfer", "jobs/v36/chamfer.chipload"),
    // Flutes (docs/specs/fluting.md): with a 6 mm ball nose 3 mm deep in two passes, one
    // ramped 25 mm at both ends and one square at its start; with a 90° V-bit 2 mm deep in
    // one pass along a half circle and a line, curved 30 mm ramps.
    ("v36_flute", "jobs/v36/flute.chipload"),
    // Fill patterns (docs/specs/fill-pattern.md) with a 90° V-bit: a cross-hatch 4 mm apart
    // at 45° inside a board round a round and a square hole, rings 3 mm apart in a circle,
    // waves 5 mm apart in a rectangle with 3 mm ramped ends in two passes.
    ("v36_pattern", "jobs/v36/pattern.chipload"),
    // Stroke patterns (docs/specs/fill-pattern.md §14) with the 90° V-bit, 0.6 mm deep, ends
    // ramped: dashes at 30° round the board's holes and scales, whole strokes only; a
    // sunburst in the circle; herringbone clipped at the rectangle's edge.
    ("v36_pattern_strokes", "jobs/v36/pattern_strokes.chipload"),
    // Prisms (docs/specs/prism.md): a 12 × 60 rectangle, a five-pointed star and a ring with
    // a pointed 90° V-bit in one pass (the star's inner corners reach the cone: a note);
    // a 44 × 70 rectangle with a 60° V-bit with a 0.5 mm flat, at most 4 mm deep, in two
    // passes: bevelled with a flat top.
    ("v36_prism", "jobs/v36/prism.chipload"),
    // Mouldings (docs/specs/moulding.md) with a 6 mm ball nose, 0.6 mm stepover: an ogee
    // 12 × 6 outside a 100 × 60 rectangle in levels of 2 mm; a bead along a line; a cove
    // along a line with curved 30 mm tapered ends in levels; a drawn profile (a flat, a
    // cove and a wall) along an arc with round ends.
    ("v36_moulding", "jobs/v36/moulding.chipload"),
    // A V-carve inlay (docs/specs/vcarve-inlay.md): "AO" in FreeSerif 30 mm high, a pocket
    // 5 mm deep in the base, and in setup 2 the plug board, 10 mm thick and fitted round the
    // artwork, carved with its outline 3 mm down and a 1.5 mm glue gap; the plug's part is
    // mirrored left to right. Both with a floor pass of the 3 mm end mill. Made with
    // `new`, `text add "AO" --font FreeSerif --height 30`, the text corpus's tools and a
    // 3 mm end mill, `op add-inlay --text 1 --tool 2 --floor-tool 3`.
    ("v36_inlay", "jobs/v36/inlay.chipload"),
    // A relief (docs/specs/stl-relief.md): a 16-sided dome 40 mm wide carved 5 mm deep at
    // (10, 10) with a 3.175 mm ball nose, lines at 30°, a quarter of the tool apart, back and
    // forth. Made with `new`, `model add dome.stl --width 40 --depth 5 --at 10,10`, the ball
    // nose set as tools.1, `op add relief --model 1`, stepover 0.25 and angle 30° set.
    ("v36_relief_dome", "jobs/v36/relief_dome.chipload"),
    // Made from relief_dome with `model set 1 --keep 50%`: its top half, 5 mm deep.
    ("v36_relief_dome_cut", "jobs/v36/relief_dome_cut.chipload"),
    // Made from relief_dome with `relief add --model 1 --tool 2 --rough-tool 1 --around
    // background --margin 8 --corner 4 --keep 60% --depth 6`, before reliefs had rims (a
    // background pocket, the model its island).
    ("v36_relief_wizard", "jobs/v36/relief_wizard.chipload"),
    // A rim (docs/specs/stl-relief.md §16): relief_dome with a flat 4 mm round the outline
    // and a 45° ramp up to the stock top, `set operations.0.rim={"shape":"outline",
    // "width":4,"ramp":0.785…}`; the fitted board grows round it.
    ("v36_relief_rim", "jobs/v36/relief_rim.chipload"),
    // The same rim filled out to the outline's convex hull (docs/specs/stl-relief.md
    // §16.7): relief_rim with `set operations.0.rim.hull=1`.
    ("v36_relief_rim_hull", "jobs/v36/relief_rim_hull.chipload"),
    // A relief from a picture read as a depth map (docs/specs/picture-relief.md):
    // tests/corpus/images/depth_dome.png (a soft grey dome on black) on relief_dome's
    // board and tools, `relief add … --width 60 --depth 4 --tool 2 --rough-tool 1
    // --smooth`, then `model set 1 --background 10% --gamma 1.5`.
    ("v36_picture_relief", "jobs/v36/picture_relief.chipload"),
    // A picture's shapes inflated (docs/specs/inflated-shapes.md): tests/corpus/images/
    // logo.png on relief_dome's board and tools, `relief add … --inflate --width 60
    // --depth 3 --tool 2 --margin 3` (round, reach and threshold automatic).
    ("v36_inflated_relief", "jobs/v36/inflated_relief.chipload"),
    // A box rim, roughed: relief_dome_rough with `set operations.0.rim={"shape":"box",
    // "width":6,"ramp":1.047…,"corner_radius":3}` (60°).
    ("v36_relief_box_rough", "jobs/v36/relief_box_rough.chipload"),
    // The same, smoothed (docs/specs/stl-relief.md §17): relief_box_rough with `op
    // add-smoothing --op 1`, the roughing end mill again, lines at 120° (across the
    // relief's 30°), 15 %, 0.5 mm left.
    (
        "v36_relief_box_smooth",
        "jobs/v36/relief_box_smooth.chipload",
    ),
    // Bull noses (docs/specs/corner-radius.md): relief_box_smooth with `set
    // tools.0.corner_radius=1` (the roughing and smoothing end mill) and the finishing tool
    // made a 6.35 mm end mill with a 1.5 mm corner (`set tools.1.type=endmill`,
    // `tools.1.diameter=6.35`, `tools.1.corner_radius=1.5`).
    ("v36_relief_bull", "jobs/v36/relief_bull.chipload"),
    // The same relief roughed first (docs/specs/stl-relief.md §14 step 3): the 6 mm end
    // mill in levels 3 mm apart, 0.4 of the tool between lines, 0.5 mm left on the dome.
    // Made from relief_dome with `op add-roughing --op 1 --tool 1`.
    (
        "v36_relief_dome_rough",
        "jobs/v36/relief_dome_rough.chipload",
    ),
    // v21_copies_by_part frozen at v37, its 3 × 2 grid cut copy by copy: v38 makes the
    // grid five instances listed in the order they were cut (2, 3, 6, 5, 4) and the order
    // `by_instance`; the G-code stays the same (docs/specs/instances.md).
    ("v37_copies", "jobs/v37/copies.chipload"),
    // The SVG badge frozen at v38 with two instances, one turned 90°, one mirrored left to
    // right and turned 180° (`part instance add --angle 90`, `--mirror left-right --angle
    // 180`): v39 gives the part itself angle 0 and at (0, 0), so it is cut where drawn
    // and the G-code stays the same (docs/specs/part-placement.md §8).
    ("v38_instances", "jobs/v38/instances.chipload"),
    // v38_instances frozen at v39 with a surface of the whole stock added first (`op
    // add-surface --depth 0.5`): v40 makes it a board surface, cut once whatever the
    // instances, so the G-code stays the same (docs/specs/surface-levels.md §4).
    (
        "v39_surface_instances",
        "jobs/v39/surface_instances.chipload",
    ),
    // nest_badge frozen at v40 after `nest --board 600x400` (so the part itself is placed
    // at 6.3, 82.3 and instance 3 stays mirrored, the setup keeps its nest), then `setup
    // add --back-of 1` and on the back's part `op add pocket --layer Pocket --depth 2`.
    // v41 moves every placement into its setup as instances; the G-code, written by the
    // schema 40 Chipload, stays the same (docs/specs/parts-and-setups.md §5).
    ("v40_placed_back", "jobs/v40/placed_back.chipload"),
    // Drawn splines (docs/specs/splines.md), made with `new --thickness 12 --size 170x110`
    // from an empty settings folder, two `shape add … spline` (a closed one of six points
    // and an open one of four), then `op add pocket --shapes 1 --depth 2`, `op add path
    // --shapes 2 --depth 4` and `op add profile --shapes 1 --through`.
    ("v42_spline", "jobs/v42/spline.chipload"),
    // Rounded corners kept as radii (docs/specs/polyline-radii.md), made with `new
    // --thickness 12 --size 170x110` from an empty settings folder, `shape add … polyline`
    // twice with `--radii` (an outline of five corners rounded 6, 6, 15, 4, 6 and a pocket
    // with three of its four rounded), `op add pocket --shapes 2 --depth 3` and `op add
    // profile --shapes 1 --through`.
    ("v42_rounded", "jobs/v42/rounded.chipload"),
    // Per-corner radii on a rectangle and a polygon (docs/specs/polyline-radii.md §4a), made
    // with `new --thickness 12 --size 180x110` from an empty settings folder, `shape add …
    // rectangle --radii "20 3 20 3"` and `polygon --sides 6 --radii "6 0 6 0 6 0"`, then
    // `op add pocket --shapes 2 --depth 3` and `op add profile --shapes 1 --through`.
    ("v42_rounded_shapes", "jobs/v42/rounded_shapes.chipload"),
    // The guide's first part (docs/book/src/first-part.md), made with its command-line
    // steps from an empty settings folder: a 160 x 100 plate with 8 mm corners and four
    // holes of radius 4, cut with `op add profile --shapes 1 2 3 4 5 --through`. Also the
    // guide's toolpath picture.
    ("v42_plate", "jobs/v42/plate.chipload"),
    // Rounds beside arcs (docs/specs/fillet-between-shapes.md §4), made with `new
    // --thickness 12 --size 170x110` from an empty settings folder, `shape add … polyline
    // --points "-70,-40 70,-40 70,40 -70,40" --bulges "0 0.4 -0.3 0" --radii "6 8 5 6"
    // --closed` (a bracket: a line into an arc, arc into arc) and a D `--points "-40,-20
    // 0,-20 0,20 -40,20" --bulges "0 0.5 0 0" --radii "0 3 3 0" --closed`, then `op add
    // pocket --shapes 2 --depth 3` and `op add profile --shapes 1 --through`.
    ("v43_bracket", "jobs/v43/bracket.chipload"),
    // A round plaque (docs/specs/text-on-arc.md §6), made with `new --thickness 12 --size
    // 200x200` from an empty settings folder, `tool add "V 60" --type vbit --diameter 12.7
    // --angle 60 --tip 0.2 --rpm 18000`, `tool use "V 60"`, then `sign add … "MAKER
    // SPACE\nEST. 2026" --height 12 --frame oval --round --borders 1 --vbit 2 --end-mill 1
    // --thickness 12`: the words on arcs, V-carved, the circle round their ring.
    ("v43_round_plaque", "jobs/v43/round_plaque.chipload"),
];

/// Projects nested before they are run (docs/specs/nesting.md §13): `chipload nest` with
/// these arguments, then `chipload run`. Nesting is deterministic, so the G-code pins the
/// placer too. The frozen file is the project before nesting.
const NEST_GOLDEN: [(&str, &str, &[&str]); 2] = [
    // v38_instances frozen at v40 on a 400 × 300 board (`set setups.0.stock.x=400
    // setups.0.stock.y=300`): its three instances (one turned 90°, one mirrored and turned
    // 180°) and two more (`--count 1=5`), quarter turns allowed, the default gap and margin.
    (
        "v40_nest_badge",
        "jobs/v40/nest_badge.chipload",
        &["--count", "1=5"],
    ),
    // A test project for the user (2026-11-17), made with `new`, `set` (a 600 × 400 × 18 mm
    // board), `shape add`, `part add`, `op add profile` and `part set 6 --thickness 6mm`:
    // six parts each cut out, two with holes, the last asking for 6 mm. Nested with counts
    // and onto more boards (docs/specs/parts-and-setups.md §9): the overflow on Setup 2,
    // like the first, and Bottom on Setup 3 of 6 mm; a program for each.
    (
        "v41_nesting_demo",
        "jobs/v41/nesting_demo.chipload",
        &[
            "--count",
            "1=4",
            "--count",
            "2=8",
            "--count",
            "3=6",
            "--count",
            "4=6",
            "--count",
            "5=10",
            "--overflow",
            "new-boards",
        ],
    ),
];

/// Projects in pieces (docs/specs/pieces.md): a program per piece, run as `chipload run`
/// does. Kept apart from GOLDEN, whose other tests plan a project as one program.
const PIECES_GOLDEN: [(&str, &str); 1] = [
    // A 900 × 500 mm rounded board cut out, a pocket and the word PUZZLE V-carved across its
    // middle, in knob pieces on 600 × 600 mm boards: two pieces, A1 and A2, each with its
    // end mill and V-bit files. Made with `new`, `shape add`, `text add`, `op add` and
    // `setup pieces --board 600x600`.
    ("v37_pieces", "jobs/v37/pieces.chipload"),
];

fn corpus(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/corpus")
        .join(rel)
}

fn chipload(args: &[&OsStr]) -> String {
    // An empty settings folder: tests never read the user's tool library.
    let settings =
        std::env::temp_dir().join(format!("chipload-golden-settings-{}", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_chipload"))
        .env("CHIPLOAD_CONFIG_DIR", settings)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Run the project to `dir/<name>.nc`: the files written, as (file name, G-code), in name
/// order. One file, or one per tool group (`<name>-1-T2.nc`, …).
/// The first line where `actual` differs from `expected`, with a few lines round it.
fn first_difference(expected: &str, actual: &str) -> String {
    let (e, a): (Vec<&str>, Vec<&str>) = (expected.lines().collect(), actual.lines().collect());
    let at = e
        .iter()
        .zip(&a)
        .position(|(x, y)| x != y)
        .unwrap_or(e.len().min(a.len()));
    let from = at.saturating_sub(3);
    let show = |lines: &[&str]| {
        lines
            .get(from..(at + 4).min(lines.len()))
            .unwrap_or(&[])
            .join("\n")
    };
    format!(
        "first difference at line {} of {} (expected) and {} (actual)\n--- expected\n{}\n--- actual\n{}",
        at + 1,
        e.len(),
        a.len(),
        show(&e),
        show(&a)
    )
}

fn run(project: &Path, dir: &Path, name: &str) -> Vec<(String, String)> {
    let out = dir.join(format!("{name}.nc"));
    chipload(&[
        "run".as_ref(),
        project.as_os_str(),
        "-o".as_ref(),
        out.as_os_str(),
        // The sheets have golden files of their own (chipload-pipeline tests/sheet.rs).
        "--no-sheet".as_ref(),
    ]);
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f == &format!("{name}.nc") || f.starts_with(&format!("{name}-")))
        .map(|f| {
            let text = std::fs::read_to_string(dir.join(&f)).unwrap();
            (f, text)
        })
        .collect();
    files.sort();
    files
}

#[test]
fn golden_gcode() {
    let base = std::env::temp_dir().join(format!("chipload-golden-{}", std::process::id()));
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in GOLDEN {
        let project = corpus(project);
        let (one, two) = (
            base.join(format!("{name}-a")),
            base.join(format!("{name}-b")),
        );
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&two).unwrap();
        let first = run(&project, &one, name);
        let second = run(&project, &two, name);
        assert_eq!(first, second, "{name}: output differs between runs");
        assert!(!first.is_empty(), "{name}: no G-code written");
        for (file, text) in &first {
            let expected = corpus(&format!("expected/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                mismatches.push(file.clone());
            }
        }
    }
    std::fs::remove_dir_all(&base).unwrap();
    assert!(
        mismatches.is_empty(),
        "golden G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

#[test]
fn golden_pieces_gcode() {
    let base = std::env::temp_dir().join(format!("chipload-golden-pieces-{}", std::process::id()));
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in PIECES_GOLDEN {
        let project = corpus(project);
        let (one, two) = (
            base.join(format!("{name}-a")),
            base.join(format!("{name}-b")),
        );
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&two).unwrap();
        let first = run(&project, &one, name);
        let second = run(&project, &two, name);
        assert_eq!(first, second, "{name}: output differs between runs");
        assert!(first.len() > 1, "{name}: a program per piece expected");
        for (file, text) in &first {
            let expected = corpus(&format!("expected/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                mismatches.push(file.clone());
            }
        }
    }
    std::fs::remove_dir_all(&base).unwrap();
    assert!(
        mismatches.is_empty(),
        "golden G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

#[test]
fn golden_nest_gcode() {
    let base = std::env::temp_dir().join(format!("chipload-golden-nest-{}", std::process::id()));
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project, args) in NEST_GOLDEN {
        let mut runs = Vec::new();
        for side in ["a", "b"] {
            let dir = base.join(format!("{name}-{side}"));
            std::fs::create_dir_all(&dir).unwrap();
            let nested = dir.join(format!("{name}.chipload"));
            std::fs::copy(corpus(project), &nested).unwrap();
            let mut nest: Vec<&OsStr> = vec!["nest".as_ref(), nested.as_os_str()];
            nest.extend(args.iter().map(|a| OsStr::new(*a)));
            chipload(&nest);
            runs.push(run(&nested, &dir, name));
        }
        assert_eq!(runs[0], runs[1], "{name}: output differs between runs");
        assert!(!runs[0].is_empty(), "{name}: no G-code written");
        for (file, text) in &runs[0] {
            let expected = corpus(&format!("expected/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                // Where it differs, and where the nest put each instance: a platform's
                // difference shows in the log (the release builds on macOS, 2026-11-18).
                let wanted = std::fs::read_to_string(&expected).unwrap_or_default();
                eprintln!("{file}: {}", first_difference(&wanted, text));
                let nested = base
                    .join(format!("{name}-a"))
                    .join(format!("{name}.chipload"));
                let placed = chipload(&[
                    "get".as_ref(),
                    nested.as_os_str(),
                    "setups".as_ref(),
                    "--json".as_ref(),
                ]);
                eprintln!("{name}: the setups as nested:\n{placed}");
                mismatches.push(file.clone());
            }
        }
    }
    std::fs::remove_dir_all(&base).unwrap();
    assert!(
        mismatches.is_empty(),
        "golden G-code differs for {mismatches:?}; if intended, regenerate with          CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

/// The same project rendered with its post but `arcs = false`: what run_job does, with
/// that one setting changed. (file name, G-code) for each file, as `run` names them.
fn render_without_arcs(project: &Path, name: &str) -> Vec<(String, String)> {
    let job = chipload_job::load_job(project).unwrap();
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let mut post = chipload_pipeline::posts::job_post(&job).unwrap();
    post.arcs = false;
    let (files, _) =
        chipload_pipeline::post_and_check(&job, &post, &plan.program, &plan.groups).unwrap();
    files
        .into_iter()
        .map(|f| {
            let file = format!("{name}{}.nc", f.suffix.unwrap_or_default());
            (file, f.gcode)
        })
        .collect()
}

#[test]
fn without_arcs_the_output_is_the_line_only_golden() {
    // docs/specs/arc-output.md: `arcs = false` must give exactly the G-code from before arcs
    // (tests/corpus/expected/lines/, frozen when arcs were added).
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    for (name, project) in GOLDEN {
        for (file, lines) in render_without_arcs(&corpus(project), name) {
            let path = corpus(&format!("expected/lines/{file}"));
            if update {
                std::fs::write(&path, &lines).unwrap();
            }
            let golden = std::fs::read_to_string(&path).unwrap();
            assert!(lines == golden, "{file}: arcs = false changed the G-code");
            assert!(
                !lines
                    .lines()
                    .any(|l| l.starts_with("G2 ") || l.starts_with("G3 "))
            );
        }
    }
}

/// Every golden file of the GOLDEN projects (`expected/<name>.nc` or `<name>-…nc`).
fn golden_files() -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(corpus("expected"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| {
            let nested = NEST_GOLDEN.iter().map(|(name, _, _)| name);
            GOLDEN
                .iter()
                .chain(&PIECES_GOLDEN)
                .map(|(name, _)| name)
                .chain(nested)
                .any(|name| {
                    f == &format!("{name}.nc")
                        || (f.starts_with(&format!("{name}-")) && f.ends_with(".nc"))
                })
        })
        .collect();
    files.sort();
    files
}

type Xy = (f64, f64);

/// (start, end, centre) of every G2/G3 line, as written.
fn arcs_in(gcode: &str) -> Vec<(Xy, Xy, Xy)> {
    let word = |line: &str, letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse::<f64>().ok()))
    };
    let (mut x, mut y) = (0.0, 0.0);
    let mut arcs = Vec::new();
    for line in gcode.lines() {
        let (nx, ny) = (word(line, 'X').unwrap_or(x), word(line, 'Y').unwrap_or(y));
        if line.starts_with("G2 ") || line.starts_with("G3 ") {
            let (i, j) = (word(line, 'I').unwrap(), word(line, 'J').unwrap());
            arcs.push(((x, y), (nx, ny), (x + i, y + j)));
        }
        (x, y) = (nx, ny);
    }
    arcs
}

#[test]
fn every_golden_arc_is_consistent_as_written() {
    // After rounding to 3 decimals, start and end radius agree within 0.001 mm (GRBL checks
    // 0.005 mm, the independent checker 0.01 mm), and no arc is a full circle. One step of
    // the third decimal is reached exactly when it can't be less: half of a circle whose
    // diameter is an odd number of thousandths has its centre between two written values
    // (v43_round_plaque's border), so the comparison allows that step's float error.
    const WRITTEN_STEP_SLACK: f64 = 1e-9;
    let mut count = 0;
    for name in golden_files() {
        let gcode = std::fs::read_to_string(corpus(&format!("expected/{name}"))).unwrap();
        for (start, end, centre) in arcs_in(&gcode) {
            let r0 = math::hypot(start.0 - centre.0, start.1 - centre.1);
            let r1 = math::hypot(end.0 - centre.0, end.1 - centre.1);
            assert!(
                (r0 - r1).abs() <= 0.001 + WRITTEN_STEP_SLACK,
                "{name}: {start:?} -> {end:?} around {centre:?}"
            );
            assert!(start != end, "{name}: full circle in one move");
            count += 1;
        }
    }
    assert!(count > 50, "the corpus should exercise arcs, found {count}");
}

// -- inch G-code (docs/specs/inch-units.md §4.4)

/// (golden name, project) rendered with the machine's G-code in inches: the three posts,
/// arcs and helices, tabs, holes, and both ways of changing tools.
const INCH: [(&str, &str); 6] = [
    ("v7_helix", "jobs/v7/helix.chipload"),
    ("v15_linuxcnc_helix", "jobs/v15/linuxcnc_helix.chipload"),
    ("v15_mach3_helix", "jobs/v15/mach3_helix.chipload"),
    ("v18_holes", "jobs/v18/holes.chipload"),
    ("v11_two_tools", "jobs/v11/two_tools.chipload"),
    (
        "v12_two_tools_program",
        "jobs/v12/two_tools_program.chipload",
    ),
];

/// The project's G-code in `units`, as run_job writes it (checked by the independent
/// checker against the machine's units): (file name, G-code) as `run` names them.
fn render_in(project: &Path, name: &str, units: chipload_job::Units) -> Vec<(String, String)> {
    let mut job = chipload_job::load_job(project).unwrap();
    job.machine.gcode_units = units;
    let drawing = chipload_pipeline::load_drawing(&job).unwrap();
    let plan = chipload_toolpath::plan_job(&job, &drawing.contours).unwrap();
    let post = chipload_pipeline::posts::job_post(&job).unwrap();
    let (files, _) =
        chipload_pipeline::post_and_check(&job, &post, &plan.program, &plan.groups).unwrap();
    files
        .into_iter()
        .map(|f| {
            let file = format!("{name}{}.nc", f.suffix.unwrap_or_default());
            (file, f.gcode)
        })
        .collect()
}

#[test]
fn inch_golden_gcode() {
    let update = std::env::var_os("CHIPLOAD_UPDATE_GOLDEN").is_some();
    let mut mismatches = Vec::new();
    for (name, project) in INCH {
        let files = render_in(&corpus(project), name, chipload_job::Units::Inch);
        for (file, text) in &files {
            assert!(text.contains("G20 G90"), "{file}: no G20");
            assert!(!text.contains("G21"), "{file}: G21 in an inch program");
            let expected = corpus(&format!("expected/inch/{file}"));
            if update {
                std::fs::write(&expected, text).unwrap();
            } else if std::fs::read_to_string(&expected).ok().as_ref() != Some(text) {
                mismatches.push(file.clone());
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "inch G-code differs for {mismatches:?}; if intended, regenerate with \
         CHIPLOAD_UPDATE_GOLDEN=1 and explain the change in the commit message"
    );
}

/// A line's words: its letter and number, as written.
fn words(line: &str) -> Vec<(char, f64)> {
    line.split_whitespace()
        .filter_map(|w| {
            let mut chars = w.chars();
            let letter = chars.next()?;
            Some((letter, chars.as_str().parse::<f64>().ok()?))
        })
        .collect()
}

/// G64's P is a length too: written in mm to 0.01 mm, so the inch program's P0.0004
/// (0.0102 mm) reads as the mm program's P0.01. Other lines as they are.
fn blend_in_mm(line: &str, scale: f64) -> String {
    let p = words(line)
        .contains(&('G', 64.0))
        .then(|| words(line).into_iter().find(|(l, _)| *l == 'P'))
        .flatten();
    match p {
        Some((_, p)) => format!("G64 P{:.2}", p * scale),
        None => line.to_string(),
    }
}

/// One motion as a controller sees it: its G code, where it ends, its feed, and for an arc
/// its centre (all in mm).
#[derive(Debug, Clone, Copy)]
struct Motion {
    code: i64,
    end: [f64; 3],
    feed: f64,
    centre: Option<(f64, f64)>,
}

/// The program's motions, every length times `scale` (25.4 for inches); its other lines
/// (codes without motion) as text, comments left out.
fn motions(gcode: &str, scale: f64) -> (Vec<Motion>, Vec<String>) {
    let mut at = [f64::NAN; 3];
    let (mut code, mut feed) = (0, 0.0);
    let (mut moves, mut other) = (Vec::new(), Vec::new());
    for line in gcode.lines().filter(|l| !l.starts_with('(')) {
        let ws = words(line);
        let moving = ws.iter().any(|(l, _)| "XYZ".contains(*l));
        if !moving {
            other.push(blend_in_mm(&line.replace("G20", "G21"), scale));
            continue;
        }
        let start = at;
        let (mut i, mut j) = (None, None);
        for (letter, value) in ws {
            match letter {
                'G' => code = value as i64,
                'X' => at[0] = value * scale,
                'Y' => at[1] = value * scale,
                'Z' => at[2] = value * scale,
                'F' => feed = value * scale,
                'I' => i = Some(value * scale),
                'J' => j = Some(value * scale),
                _ => {}
            }
        }
        let centre = i.zip(j).map(|(i, j)| (start[0] + i, start[1] + j));
        moves.push(Motion {
            code,
            end: at,
            feed: if code == 0 { 0.0 } else { feed },
            centre,
        });
    }
    (moves, other)
}

#[test]
fn every_golden_project_moves_the_same_in_inches() {
    // The inch program is the mm program: the same codes, and every motion ends where the mm
    // one does, at the same feed, within both roundings (half a step of each; an arc's
    // centre is measured from its start as rounded and may take a neighbouring step to keep
    // its radii equal, so two steps of each). A move shorter than an inch step can vanish
    // in inches, where its end rounds to its start.
    let inch_step = 0.0001 * chipload_core::MM_PER_INCH;
    let mm_step = 0.001;
    let tolerance = (inch_step + mm_step) / 2.0 + 1e-9;
    let centre_tolerance = 2.0 * (inch_step + mm_step) + 1e-9;
    for (name, project) in GOLDEN {
        let project = corpus(project);
        let mm = render_in(&project, name, chipload_job::Units::Mm);
        let inch = render_in(&project, name, chipload_job::Units::Inch);
        assert_eq!(mm.len(), inch.len(), "{name}: number of files");
        for ((file, mm), (_, inch)) in mm.iter().zip(&inch) {
            let (a, other_a) = motions(mm, 1.0);
            let (b, other_b) = motions(inch, chipload_core::MM_PER_INCH);
            assert_eq!(other_a, other_b, "{file}: lines without motion");
            let close = |p: [f64; 3], q: [f64; 3], tol: f64| {
                p.iter()
                    .zip(&q)
                    .all(|(u, v)| (u.is_nan() && v.is_nan()) || (u - v).abs() <= tol)
            };
            let (mut i, mut j, mut skipped) = (0, 0, 0);
            while i < a.len() && j < b.len() {
                let (m, n) = (a[i], b[j]);
                let same = m.code == n.code
                    && close(m.end, n.end, tolerance)
                    && (m.feed - n.feed).abs() <= tolerance
                    && match (m.centre, n.centre) {
                        (Some(c), Some(d)) => {
                            (c.0 - d.0).abs() <= centre_tolerance
                                && (c.1 - d.1).abs() <= centre_tolerance
                        }
                        (None, None) => true,
                        _ => false,
                    };
                if same {
                    (i, j) = (i + 1, j + 1);
                    continue;
                }
                // Only a move too short to show may be missing: its end within an inch step
                // (plus the mm rounding) of the one before. The two grids don't nest, so it
                // may be missing from either program: two points can round together in mm
                // and apart in inches, or the other way round.
                let tiny = inch_step + mm_step;
                let before = |list: &[Motion], k: usize| {
                    if k > 0 {
                        list[k - 1].end
                    } else {
                        [f64::NAN; 3]
                    }
                };
                if close(m.end, before(&a, i), tiny) {
                    i += 1;
                } else if close(n.end, before(&b, j), tiny) {
                    j += 1;
                } else {
                    panic!("{file}: motion {} {m:?} vs {n:?}", i + 1);
                }
                skipped += 1;
            }
            // What's left at the end can only be such tiny moves.
            for k in i..a.len() {
                assert!(
                    close(a[k].end, a[k - 1].end, inch_step + mm_step),
                    "{file}: left over"
                );
                skipped += 1;
            }
            for k in j..b.len() {
                assert!(
                    close(b[k].end, b[k - 1].end, inch_step + mm_step),
                    "{file}: left over"
                );
                skipped += 1;
            }
            // At most 1 % of the moves (and 10 in any file): a long V-carve has thousands of
            // short moves, a few of them shorter than a rounding step.
            let most = (a.len() / 100).max(10);
            assert!(
                skipped <= most,
                "{file}: {skipped} moves vanished in inches"
            );
        }
    }
}
