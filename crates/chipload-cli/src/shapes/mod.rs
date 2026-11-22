// SPDX-License-Identifier: GPL-3.0-or-later
//! `chipload shape …`: drawn shapes added, changed, moved, edited (trim, offset, fillet, arrays,
//! …) and named.

use super::*;

mod edits;
mod geometry;

pub(crate) use edits::*;
pub(crate) use geometry::*;

#[derive(Subcommand)]
pub(crate) enum ShapeAction {
    /// The shape library: save, insert, list (docs/specs/shape-library.md §7).
    #[command(flatten)]
    Library(shape_library::LibraryCommand),
    /// The shapes like one (docs/specs/fewer-clicks.md §3): for a circle, every circle of
    /// the same diameter on any layer; else the shapes of the same kind (closed or open) on
    /// the same layer whose boxes are the same size, either way round. Prints their ids,
    /// the shape itself included, for `op add … --shapes`.
    Similar {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
    },
    /// The closed shapes inside a closed shape, nested ones too, without it
    /// (docs/specs/fewer-clicks.md §3): a part's pockets and holes. Prints their ids.
    Inside {
        /// The project (.chipload).
        project: PathBuf,
        /// The outline's shape id (as `inspect` lists it, without the S).
        id: u64,
    },
    /// Add a drawn shape; prints its id.
    Add {
        /// The project (.chipload).
        project: PathBuf,
        kind: KindArg,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Change a drawn shape's values or layer; it keeps its id.
    Set {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        #[command(flatten)]
        values: ShapeValues,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Remove a drawn shape. Operations that used it are flagged.
    #[command(alias = "delete")]
    Remove {
        /// The project (.chipload).
        project: PathBuf,
        id: u64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copy drawn shapes, moved by --dx and --dy, on their layers; prints the new ids.
    Copy {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move the copies right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move the copies up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Mirror drawn shapes in place, about the middle of the box round them. They keep
    /// their ids, so the operations that cut them follow.
    Mirror {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// x: flip the X coordinates (left becomes right); y: flip Y (up becomes down), about
        /// the middle of the box round them.
        #[arg(long, required_unless_present_any = ["line", "about_shape"], conflicts_with_all = ["line", "about_shape", "copy"])]
        axis: Option<AxisArg>,
        /// Mirror about the line through two points "X,Y X,Y" (docs/specs/drawing-improvements.md §1).
        #[arg(long, allow_hyphen_values = true, conflicts_with = "about_shape")]
        line: Option<String>,
        /// Mirror about drawn shape ID, one straight line (a construction centre line).
        #[arg(long)]
        about_shape: Option<u64>,
        /// Keep the shapes and add their mirror images; an open shape with both ends on the
        /// line joins its image into one closed shape.
        #[arg(long)]
        copy: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Move drawn shapes by --dx and --dy; they keep their ids, so the operations that cut
    /// them follow (docs/specs/drawn-shapes-box.md).
    Move {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far to move them right, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dx: f64,
        /// How far to move them up, mm.
        #[arg(value_parser = crate::lengths::length, long, allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Turn drawn shapes counter-clockwise about the middle of the box round them, or a
    /// point.
    Turn {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The angle, degrees, counter-clockwise; negative turns clockwise.
        #[arg(long, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Turn about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Scale drawn shapes evenly about the middle of the box round them, or a point; or by
    /// reference: two points and the length their distance should have, the first point
    /// staying (docs/specs/scale-and-stretch.md §2).
    Scale {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The factor, greater than 0: 2 doubles them.
        #[arg(
            long,
            required_unless_present = "reference",
            conflicts_with = "reference"
        )]
        factor: Option<f64>,
        /// Scale about this point "X,Y" instead.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true, conflicts_with = "reference")]
        about: Option<[f64; 2]>,
        /// Two points "X1,Y1 X2,Y2", on any shape: scaled so their distance becomes --length,
        /// the first point staying.
        #[arg(long, allow_hyphen_values = true, requires = "length")]
        reference: Option<String>,
        /// With --reference: the length the two points' distance becomes, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long, requires = "reference")]
        length: Option<f64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Stretch drawn shapes along X and Y about the middle of the box round them, or a point
    /// (docs/specs/scale-and-stretch.md §3). Circles and arcs become polylines fitted to the
    /// stretched curve; corner radii stay. Prints the shapes that became polylines.
    Stretch {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// The factor along X, greater than 0 (default 1).
        #[arg(long, conflicts_with = "width")]
        x: Option<f64>,
        /// The factor along Y, greater than 0 (default 1).
        #[arg(long, conflicts_with = "height")]
        y: Option<f64>,
        /// Stretch along X until the box round the shapes is this wide, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        width: Option<f64>,
        /// Stretch along Y until the box round the shapes is this high, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        height: Option<f64>,
        /// The point that stays, "X,Y"; default: the middle of the box round the shapes.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        about: Option<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Add the curve at a distance from drawn shapes, on their layers; the shapes stay
    /// (docs/specs/drawing-editing.md §2). Prints the new ids.
    Offset {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How far, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Which side: outside or inside of a closed shape, left or right of an open one
        /// (default: outside, or left).
        #[arg(long, value_enum)]
        side: Option<OffsetSideArg>,
        /// Round corners (the exact offset) or sharp ones (the sides extended to meet).
        #[arg(long, value_enum, default_value = "round")]
        corners: CornersArg,
        /// How many, each the distance further than the last.
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Round corners of a drawn shape with arcs of a radius, beside arcs too; it keeps its
    /// id (docs/specs/drawing-editing.md §3). With --with, a round between two open drawn
    /// polylines, lines or arcs instead, both trimmed or extended to it and joined as after
    /// a trim (docs/specs/fillet-between-shapes.md §5).
    Fillet {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The radius, mm (or with a unit); 0 makes a corner sharp, or two shapes meet in
        /// one.
        #[arg(value_parser = crate::lengths::length, long)]
        radius: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long, conflicts_with = "with")]
        corner: Option<usize>,
        /// The second shape: a round between the two.
        #[arg(long, requires = "keep")]
        with: Option<u64>,
        /// With --with, twice: a point near the part of each shape to keep, "X,Y", in the
        /// shapes' order.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
              num_args = 1, requires = "with")]
        keep: Vec<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Trim a drawn shape: the piece of it at --at, between the nearest crossings with the
    /// shapes shown, goes; the rest keeps the id, a second piece is a new shape
    /// (docs/specs/drawing-editing.md §4).
    Trim {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// A point on (or near) the piece to remove, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Extend an open drawn shape's straight end along itself to the nearest shape shown
    /// ahead (docs/specs/drawing-editing.md §4).
    Extend {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// Which end.
        #[arg(long, value_enum)]
        end: EndArg,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Break a drawn shape at the point of it nearest --at: an open one becomes two, a
    /// closed one opens there (docs/specs/drawing-editing.md §4).
    Break {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// The point, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        at: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join closed drawn shapes into one: everything any of them covers. The result takes
    /// the first shape's id; the others go (docs/specs/drawing-editing.md §5).
    Union {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut closed drawn shapes out of another: BASE keeps its id, the cutters go
    /// (docs/specs/drawing-editing.md §5).
    Subtract {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape cut (as `inspect` lists it, without the S).
        base: u64,
        /// The shapes cut out of it.
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// What closed drawn shapes have in common: the result takes the first shape's id; the
    /// others go (docs/specs/drawing-editing.md §5).
    Intersect {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, two or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 2..)]
        ids: Vec<u64>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copies of drawn shapes in a grid, on their layers; the originals stay
    /// (docs/specs/drawing-editing.md §6). Prints the new ids.
    Array {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Columns, the originals' included.
        #[arg(long, default_value_t = 1)]
        columns: u32,
        /// Rows, the originals' included.
        #[arg(long, default_value_t = 1)]
        rows: u32,
        /// From one column to the next, mm (or with a unit); negative goes left.
        #[arg(value_parser = crate::lengths::length, long, default_value = "0", allow_hyphen_values = true)]
        dx: f64,
        /// From one row to the next, mm (or with a unit); negative goes down.
        #[arg(value_parser = crate::lengths::length, long, default_value = "0", allow_hyphen_values = true)]
        dy: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// A line of copies of drawn shapes in any direction, on their layers; the originals
    /// stay (docs/specs/compass.md §8). Prints the new ids.
    ArrayLinear {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How many, the originals included.
        #[arg(long)]
        count: u32,
        /// From one copy to the next, "DX,DY" in mm (or with a unit).
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        step: [f64; 2],
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Copies of drawn shapes round a centre, on their layers; the originals stay
    /// (docs/specs/drawing-editing.md §6). Prints the new ids.
    ArrayPolar {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S).
        #[arg(required = true)]
        ids: Vec<u64>,
        /// How many, the originals included.
        #[arg(long)]
        count: u32,
        /// The centre, "X,Y".
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true)]
        center: [f64; 2],
        /// The sweep, degrees: 360 spreads them evenly round, less puts the first and last
        /// at its ends; negative goes clockwise.
        #[arg(long, default_value_t = 360.0, allow_hyphen_values = true)]
        angle_deg: f64,
        /// Move the copies round without turning them.
        #[arg(long)]
        no_rotate: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Line up drawn shapes: each one's box side or middle with that of the first shape
    /// named, which stays put, or the stock's; they move whole and keep their ids. The
    /// shapes of one contour, and a group named whole, line up as one
    /// (docs/specs/drawing-editing.md §7, arrange-whole.md, align-first.md).
    Align {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S); the first stays put.
        #[arg(required = true)]
        ids: Vec<u64>,
        /// Which side or middle lines up.
        #[arg(long, value_enum)]
        to: AlignToArg,
        /// Line up with the stock instead (then one shape is enough).
        #[arg(long)]
        stock: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Even out the gaps between drawn shapes' boxes, the first and last staying put; the
    /// shapes of one contour, and a group named whole, count as one
    /// (docs/specs/drawing-editing.md §7, arrange-whole.md).
    Distribute {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids, three or more (as `inspect` lists them, without the S).
        #[arg(required = true, num_args = 3..)]
        ids: Vec<u64>,
        /// Along X.
        #[arg(
            long,
            conflicts_with = "vertical",
            required_unless_present = "vertical"
        )]
        horizontal: bool,
        /// Along Y.
        #[arg(long)]
        vertical: bool,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Cut corners of a drawn shape straight across, a distance back along each side; it
    /// keeps its id (docs/specs/drawing-editing.md §3).
    Chamfer {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape id (as `inspect` lists it, without the S).
        id: u64,
        /// How far back along each side, mm (or with a unit).
        #[arg(value_parser = crate::lengths::length, long)]
        distance: f64,
        /// Only this corner: its point's number, from 0 (default: every corner).
        #[arg(long, conflicts_with = "with")]
        corner: Option<usize>,
        /// The second shape: a straight cut between two lines, back from where they meet
        /// (docs/specs/fillet-between-shapes.md §5).
        #[arg(long, requires = "keep")]
        with: Option<u64>,
        /// With --with, twice: a point near the part of each line to keep, "X,Y", in the
        /// shapes' order.
        #[arg(long, value_parser = crate::lengths::point, allow_hyphen_values = true,
              num_args = 1, requires = "with")]
        keep: Vec<[f64; 2]>,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Join open drawn polylines whose ends are closer than --gap into one, on the same
    /// layer; a chain that comes back to its start closes (docs/specs/explode-and-join.md).
    Join {
        /// The project (.chipload).
        project: PathBuf,
        /// The shape ids (as `inspect` lists them, without the S); none: every drawn shape.
        ids: Vec<u64>,
        /// Join ends closer than this, 0.001 to 1 mm.
        #[arg(long, value_parser = crate::lengths::length, default_value = "0.01")]
        gap: f64,
        /// Where to write the result; default: the project itself.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AxisArg {
    X,
    Y,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum OffsetSideArg {
    Outside,
    Inside,
    Left,
    Right,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum AlignToArg {
    Left,
    #[value(alias = "center")]
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum EndArg {
    Start,
    End,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum CornersArg {
    Round,
    Sharp,
}

#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    Circle,
    Rectangle,
    Polyline,
    Polygon,
    /// The smooth curve through --points (docs/specs/splines.md).
    Spline,
    /// A slot bent round --center at --radius from --start-deg to --end-deg, --width wide: a
    /// closed polyline of arcs (docs/specs/drawing-improvements.md §3).
    ArcSlot,
}

/// Defining values, all in mm; the angle in degrees.
#[derive(clap::Args)]
pub(crate) struct ShapeValues {
    /// Centre "X,Y" (circle, rectangle, polygon).
    #[arg(long, allow_hyphen_values = true)]
    center: Option<String>,
    /// A circle's radius; a polygon's, to its corners.
    #[arg(value_parser = crate::lengths::length, long)]
    radius: Option<f64>,
    /// A polygon's number of sides, 3 to 1000.
    #[arg(long)]
    sides: Option<u32>,
    /// A polygon's size across its flats (twice the radius to the middle of a side), in
    /// place of --radius.
    #[arg(value_parser = crate::lengths::length, long, conflicts_with = "radius")]
    across_flats: Option<f64>,
    /// Along the rectangle's own X axis.
    #[arg(value_parser = crate::lengths::length, long)]
    length: Option<f64>,
    /// Along the rectangle's own Y axis.
    #[arg(value_parser = crate::lengths::length, long)]
    width: Option<f64>,
    /// Rotation about the centre (a polygon: its first corner's angle from +X), degrees,
    /// counter-clockwise.
    #[arg(long, allow_hyphen_values = true)]
    angle_deg: Option<f64>,
    /// The radius of the rectangle's corners, at most half its shorter side (default 0:
    /// sharp).
    #[arg(value_parser = crate::lengths::length, long)]
    corner_radius: Option<f64>,
    /// Polyline points, or the points a spline passes through: "X,Y X,Y ...".
    #[arg(long, allow_hyphen_values = true)]
    points: Option<String>,
    /// Polyline bulges "B B ...", one per segment: tan(sweep / 4) as in DXF, 0 straight,
    /// 1 a counter-clockwise half circle, -1 clockwise (default: all straight); t: an arc
    /// going on smoothly from the segment before it (docs/specs/polyline-tangent-arcs.md).
    #[arg(long, allow_hyphen_values = true)]
    bulges: Option<String>,
    /// An arc slot's start, degrees counter-clockwise from +X.
    #[arg(long, allow_hyphen_values = true)]
    start_deg: Option<f64>,
    /// An arc slot's end, degrees: past the start goes counter-clockwise, before it
    /// clockwise.
    #[arg(long, allow_hyphen_values = true)]
    end_deg: Option<f64>,
    /// Corner radii "R R ...", mm, 0 sharp: a polyline's one per point (an open one's ends
    /// 0), a rectangle's four counter-clockwise from its (−length/2, −width/2) corner, a
    /// polygon's one per corner from its first. Each corner is rounded by an arc tangent to
    /// both sides, its point staying at the sharp corner (docs/specs/polyline-radii.md).
    #[arg(long, allow_hyphen_values = true)]
    radii: Option<String>,
    /// The polyline or spline doesn't return to its first point.
    #[arg(long, conflicts_with = "closed")]
    open: bool,
    /// The polyline or spline returns to its first point (the default for a new polyline;
    /// a new spline is open without it).
    #[arg(long)]
    closed: bool,
    #[arg(long)]
    layer: Option<String>,
    /// Its name in the Browser; "" takes the name away.
    #[arg(long)]
    name: Option<String>,
    /// Construction geometry: shown and snapped to, never cut (`--construction false`
    /// makes it an ordinary shape again).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    construction: Option<bool>,
}
