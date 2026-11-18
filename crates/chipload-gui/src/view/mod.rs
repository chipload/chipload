// SPDX-License-Identifier: GPL-3.0-or-later
//! The drawing canvas: world (mm, Y up) to screen transform, what to draw, and picking.
//! No geometry decisions here: it only shows core results.

use chipload_core::Point;
use chipload_core::math;
use eframe::egui::{Color32, Pos2, Rect, Vec2};

mod legend;
mod paint;
mod progress;
mod scene;
mod shapes;
mod sim;
#[cfg(test)]
mod tests;

pub use legend::{Swatch, legend, paint_swatch};
pub use progress::{Progress, ToolLook};
pub use scene::{
    COMPASS_ARM_END_PX, COMPASS_ARM_START_PX, COMPASS_CENTRE_PX, COMPASS_RING_PX, CompassShape,
    CornerMarker, InstanceGhost, Scene, Sheet, StartMarker, TURN_HANDLE_PX, TabMarker, badge_rect,
};
#[cfg(test)]
pub use shapes::Kind;
pub use shapes::{ContourShape, Highlight, ToolpathShape};
pub use sim::{PictureLayer, SimLayer, SimSurface};

/// The canvas's colours, one set for each theme (docs/specs/browser-and-layout.md §6b).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// The background: a gradient from the top colour to the bottom one (`background`).
    pub background_top: Color32,
    pub background: Color32,
    /// The grid's lines: every step, and every tenth.
    pub grid_minor: Color32,
    pub grid_major: Color32,
    pub outer: Color32,
    pub hole: Color32,
    pub open: Color32,
    pub invalid: Color32,
    pub stock: Color32,
    pub feed: Color32,
    pub feed_rejected: Color32,
    pub rapid: Color32,
    pub label: Color32,
    pub axis_x: Color32,
    pub axis_y: Color32,
    pub axis_z: Color32,
    pub selected: Color32,
    /// The snap marker: a point of the drawing the cursor is on.
    pub snap: Color32,
    /// View → Measure's line and text, and the box behind the text.
    pub measure: Color32,
    pub measure_back: Color32,
    /// Middle handles of a polyline's segments: they insert a point or bend the segment,
    /// unlike the square point handles, which move a point.
    pub middle_handle: Color32,
    /// Inside the handles.
    pub handle_fill: Color32,
    pub tab: Color32,
    /// Copies of the part: outlines, and the toolpath's colour faded by GHOST_FADE.
    pub ghost: Color32,
    /// Contours of the operation selected in the Browser, and its rings' start markers.
    pub operation: Color32,
    /// The playback's tool.
    pub tool_mark: Color32,
    /// What the highest cut in 3D is shaded towards (the deepest is the plain colour).
    pub shade_toward: Color32,
}

impl Palette {
    /// The dark canvas.
    pub const DARK: Palette = Palette {
        background_top: Color32::from_rgb(0x1f, 0x21, 0x24),
        background: Color32::from_rgb(0x16, 0x17, 0x1a),
        grid_minor: Color32::from_rgba_premultiplied(9, 9, 9, 9),
        grid_major: Color32::from_rgba_premultiplied(22, 22, 22, 22),
        outer: Color32::from_rgb(0xe8, 0xea, 0xed),
        hole: Color32::from_rgb(0x8a, 0xb4, 0xf8),
        open: Color32::from_rgb(0xfb, 0xbc, 0x04),
        invalid: Color32::from_rgb(0xf2, 0x8b, 0x82),
        stock: Color32::from_rgb(0x80, 0x86, 0x8b),
        feed: Color32::from_rgb(0x34, 0xa8, 0x53),
        feed_rejected: Color32::from_rgb(0xea, 0x43, 0x35),
        rapid: Color32::from_rgb(0x9a, 0xa0, 0xa6),
        label: Color32::from_rgb(0xfd, 0xd6, 0x63),
        axis_x: Color32::from_rgb(0xf2, 0x8b, 0x82),
        axis_y: Color32::from_rgb(0x81, 0xc9, 0x95),
        axis_z: Color32::from_rgb(0x8a, 0xb4, 0xf8),
        selected: Color32::from_rgb(0xff, 0x8b, 0xcb),
        snap: Color32::from_rgb(0xfd, 0xd6, 0x63),
        measure: Color32::from_rgb(0x7f, 0xe8, 0xff),
        measure_back: Color32::from_black_alpha(190),
        middle_handle: Color32::from_rgb(0x78, 0xd9, 0xec),
        handle_fill: Color32::from_gray(20),
        tab: Color32::from_rgb(0xff, 0x9e, 0x40),
        ghost: Color32::from_rgb(0x74, 0x78, 0x7e),
        operation: Color32::from_rgb(0xc5, 0x8a, 0xf9),
        tool_mark: Color32::from_rgb(0xfd, 0xd6, 0x63),
        shade_toward: Color32::WHITE,
    };

    /// The light canvas: the same meanings in darker inks.
    pub const LIGHT: Palette = Palette {
        background_top: Color32::from_rgb(0xf3, 0xf5, 0xf7),
        background: Color32::from_rgb(0xdc, 0xe0, 0xe5),
        grid_minor: Color32::from_rgba_premultiplied(0, 0, 0, 14),
        grid_major: Color32::from_rgba_premultiplied(0, 0, 0, 34),
        outer: Color32::from_rgb(0x20, 0x21, 0x24),
        hole: Color32::from_rgb(0x1a, 0x56, 0xc4),
        open: Color32::from_rgb(0x8a, 0x4b, 0x00),
        invalid: Color32::from_rgb(0xb3, 0x26, 0x1e),
        stock: Color32::from_rgb(0x5f, 0x63, 0x68),
        feed: Color32::from_rgb(0x0a, 0x4f, 0x22),
        feed_rejected: Color32::from_rgb(0xb3, 0x26, 0x1e),
        rapid: Color32::from_rgb(0x3c, 0x40, 0x43),
        label: Color32::from_rgb(0x8a, 0x4b, 0x00),
        axis_x: Color32::from_rgb(0xc5, 0x22, 0x1f),
        axis_y: Color32::from_rgb(0x13, 0x73, 0x33),
        axis_z: Color32::from_rgb(0x1a, 0x56, 0xc4),
        selected: Color32::from_rgb(0xb0, 0x0e, 0x5a),
        snap: Color32::from_rgb(0x8a, 0x4b, 0x00),
        measure: Color32::from_rgb(0x00, 0x5f, 0x70),
        measure_back: Color32::from_rgba_premultiplied(220, 220, 220, 220),
        middle_handle: Color32::from_rgb(0x00, 0x6d, 0x77),
        handle_fill: Color32::WHITE,
        tab: Color32::from_rgb(0xb3, 0x47, 0x00),
        ghost: Color32::from_rgb(0x5f, 0x63, 0x68),
        operation: Color32::from_rgb(0x7b, 0x1f, 0xa2),
        tool_mark: Color32::from_rgb(0x8a, 0x4b, 0x00),
        shade_toward: Color32::BLACK,
    };

    /// The palette for a theme.
    pub fn of(dark: bool) -> Palette {
        if dark { Palette::DARK } else { Palette::LIGHT }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette::DARK
    }
}

/// How far a stale toolpath and the copies are faded.
const GHOST_FADE: f32 = 0.65;

/// Distance between direction arrows along the cutting path.
pub const ARROW_SPACING_MM: f64 = 40.0;
const AXIS_LENGTH_MM: f64 = 15.0;
/// The radius of the work zero's handle at the foot of the axes, pixels.
const ZERO_HANDLE_PX: f32 = 5.0;
/// The side of the lock drawn beside a back's locked zero, pixels.
const LOCK_PX: f32 = 12.0;
/// The turn-over mark over the gap between a front's and its back's sheets: its side,
/// pixels, and how far left of the right-hand sheet its centre is, mm (half the pair's gap).
const PAIR_MARK_PX: f32 = 20.0;
const PAIR_MARK_MM: f64 = 5.0;
/// More lines than this across the area and the finer ones are left out.
const GRID_MAX_LINES: usize = 400;
/// In 3D the grid reaches at least this far past the stock.
const GRID_MARGIN_MM: f64 = 50.0;
/// The axis lines across the grid, against the short strokes at the zero.
const AXIS_LINE_FADE: f32 = 0.55;
/// How close (on screen) a click must be to a contour line.
pub const PICK_TOLERANCE_PX: f32 = 6.0;

/// The circle at an operation's first plunge, screen pixels; its number sits up and right.
const OP_LABEL_RADIUS: f32 = 5.0;
const OP_LABEL_TEXT_OFFSET: Vec2 = Vec2::new(7.0, -19.0);
const DASH_PX: f32 = 6.0;
/// How far the turn axis runs past the board at each end, points.
const TURN_AXIS_OVERHANG_PX: f32 = 14.0;
const GAP_PX: f32 = 4.0;
/// Construction geometry's dots, screen pixels (docs/specs/dxf-export.md §B3).
const CONSTRUCTION_DASH_PX: f32 = 2.0;
const CONSTRUCTION_GAP_PX: f32 = 4.0;

/// A point in 3D, mm (X, Y as drawn, Z as the program uses it).
pub type Point3 = (f64, f64, f64);

/// The most the 3D view tilts: just short of looking from the side, so a click still lands
/// on the stock top (docs/specs/3d-view.md section 3).
pub const MAX_TILT_RAD: f64 = 85.0 * std::f64::consts::PI / 180.0;

/// The camera: an orthographic view that turns about the vertical axis (`turn`), tilts
/// towards the viewer (`tilt`, 0 = straight down), then pans and zooms as the 2D view did.
/// With no turn and no tilt it is exactly the old 2D mapping: screen = offset + (x, −y) · scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Pixels per millimetre.
    pub scale: f64,
    /// Screen position of the world origin.
    pub offset: (f64, f64),
    /// Rotation about the vertical axis, radians.
    pub turn: f64,
    /// 0 (top view) to MAX_TILT_RAD.
    pub tilt: f64,
    /// Stretch of Z in the 3D view (1 = true scale).
    pub z_scale: f64,
    /// Where 2D items (contours, selection, material left) sit: the stock top's Z.
    pub top_z: f64,
    /// The board turned over (docs/specs/two-sided.md §10.2): every point is mirrored about
    /// its centre line and turned upside down before it is shown. None: as drawn.
    pub turned: Option<Turned>,
}

/// How the view turns the board over: about the vertical line X = `at` (left to right) or
/// the horizontal line Y = `at`, with Z turned about the board's middle: `z_sum` is its top
/// plus its bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turned {
    pub left_right: bool,
    pub at: f64,
    pub z_sum: f64,
}

impl Turned {
    /// A point as it lies on the turned board; turning twice gives it back.
    pub fn point(self, (x, y, z): Point3) -> Point3 {
        let (x, y) = self.xy((x, y));
        (x, y, self.z_sum - z)
    }

    pub fn xy(self, (x, y): Point) -> Point {
        if self.left_right {
            (2.0 * self.at - x, y)
        } else {
            (x, 2.0 * self.at - y)
        }
    }
}

/// A rectangle in camera units: (from, to) across and up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub across: (f64, f64),
    pub up: (f64, f64),
}

impl Default for View {
    fn default() -> Self {
        View {
            scale: 2.0,
            offset: (100.0, 400.0),
            turn: 0.0,
            tilt: 0.0,
            z_scale: 1.0,
            top_z: 0.0,
            turned: None,
        }
    }
}

impl View {
    pub fn is_top(self) -> bool {
        self.turn == 0.0 && self.tilt == 0.0
    }

    /// (across, up) of a world point in camera units (mm), before pan and zoom; on a
    /// turned board, as it lies turned (2D items on its face up).
    fn camera(self, p: Point3) -> (f64, f64) {
        match self.turned {
            Some(turned) => {
                // Heights from the face 2D items sit on, which is up now.
                let reference = turned.z_sum - self.top_z;
                self.camera_raw(turned.point(p), reference)
            }
            None => self.camera_raw(p, self.top_z),
        }
    }

    /// The camera on a point already as it lies, `reference` the Z at height 0.
    fn camera_raw(self, (x, y, z): Point3, reference: f64) -> (f64, f64) {
        if self.is_top() {
            return (x, y); // exactly the 2D mapping, whatever Z is
        }
        let (sin_t, cos_t) = math::sin_cos(self.turn);
        let (sin_a, cos_a) = math::sin_cos(self.tilt);
        let across = x * cos_t - y * sin_t;
        let depth = x * sin_t + y * cos_t;
        let height = (z - reference) * self.z_scale;
        (across, depth * cos_a + height * sin_a)
    }

    /// A point already as it lies on the turned board (the turned simulation's surface):
    /// shown without turning it again.
    pub fn to_screen3_raw(self, p: Point3) -> Pos2 {
        let reference = self
            .turned
            .map_or(self.top_z, |turned| turned.z_sum - self.top_z);
        let (across, up) = self.camera_raw(p, reference);
        Pos2::new(
            (self.offset.0 + across * self.scale) as f32,
            (self.offset.1 - up * self.scale) as f32,
        )
    }

    pub fn to_screen3(self, p: Point3) -> Pos2 {
        let (across, up) = self.camera(p);
        Pos2::new(
            (self.offset.0 + across * self.scale) as f32,
            (self.offset.1 - up * self.scale) as f32,
        )
    }

    /// A drawing point on the stock top.
    pub fn to_screen(self, p: Point) -> Pos2 {
        self.to_screen3((p.0, p.1, self.top_z))
    }

    /// The point on the stock top under a screen position.
    pub fn to_world(self, p: Pos2) -> Point {
        let across = (f64::from(p.x) - self.offset.0) / self.scale;
        let up = (self.offset.1 - f64::from(p.y)) / self.scale;
        let lying = if self.is_top() {
            (across, up)
        } else {
            let (sin_t, cos_t) = math::sin_cos(self.turn);
            let depth = up / math::cos(self.tilt); // on the stock top the height term is 0
            (
                across * cos_t + depth * sin_t,
                -across * sin_t + depth * cos_t,
            )
        };
        // Turned back from the board as it lies to the drawing.
        self.turned.map_or(lying, |turned| turned.xy(lying))
    }

    /// Show every point inside `screen` with a 5 % margin, in the current orientation.
    pub fn fit(&mut self, points: &[Point3], screen: Rect) {
        if points.is_empty() {
            return;
        }
        let projected: Vec<(f64, f64)> = points.iter().map(|&p| self.camera(p)).collect();
        let fold = |f: fn(f64, f64) -> f64, start: f64, pick: fn(&(f64, f64)) -> f64| {
            projected.iter().map(pick).fold(start, f)
        };
        let (x0, x1) = (
            fold(f64::min, f64::INFINITY, |p| p.0),
            fold(f64::max, f64::NEG_INFINITY, |p| p.0),
        );
        let (y0, y1) = (
            fold(f64::min, f64::INFINITY, |p| p.1),
            fold(f64::max, f64::NEG_INFINITY, |p| p.1),
        );
        let span = (x1 - x0).max(y1 - y0).max(1e-3);
        let margin = span * 0.05;
        let (w, h) = (x1 - x0 + 2.0 * margin, y1 - y0 + 2.0 * margin);
        self.scale = (f64::from(screen.width()) / w).min(f64::from(screen.height()) / h);
        let centre = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let c = screen.center();
        self.offset = (
            f64::from(c.x) - centre.0 * self.scale,
            f64::from(c.y) + centre.1 * self.scale,
        );
    }

    /// What `screen` shows, in camera units (mm across and up, in the current orientation).
    pub fn visible_area(self, screen: Rect) -> Area {
        Area {
            across: (
                (f64::from(screen.min.x) - self.offset.0) / self.scale,
                (f64::from(screen.max.x) - self.offset.0) / self.scale,
            ),
            up: (
                (self.offset.1 - f64::from(screen.max.y)) / self.scale,
                (self.offset.1 - f64::from(screen.min.y)) / self.scale,
            ),
        }
    }

    /// Show all of `area` in `screen`, centred, as large as fits: the same area stays in
    /// view when the canvas changes size (docs/specs/canvas-resize.md).
    pub fn show_area(&mut self, area: Area, screen: Rect) {
        let width = area.across.1 - area.across.0;
        let height = area.up.1 - area.up.0;
        if !(width > 0.0 && height > 0.0) || screen.width() <= 0.0 || screen.height() <= 0.0 {
            return;
        }
        self.scale = (f64::from(screen.width()) / width)
            .min(f64::from(screen.height()) / height)
            .clamp(1e-4, 1e5);
        let centre = (
            (area.across.0 + area.across.1) / 2.0,
            (area.up.0 + area.up.1) / 2.0,
        );
        let c = screen.center();
        self.offset = (
            f64::from(c.x) - centre.0 * self.scale,
            f64::from(c.y) + centre.1 * self.scale,
        );
    }

    /// Zoom by `factor`, keeping the stock-top point under `anchor` where it is.
    pub fn zoom_at(&mut self, anchor: Pos2, factor: f64) {
        let world = self.to_world(anchor);
        self.scale = (self.scale * factor).clamp(1e-4, 1e5);
        self.keep_at(world, anchor);
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.offset.0 += f64::from(delta.x);
        self.offset.1 += f64::from(delta.y);
    }

    /// Turn and tilt by a mouse drag (pixels), keeping `pivot` where it is on screen: the
    /// model turns with the mouse, as in many CAD programs (issue #16): a drag to the right
    /// brings its front to the right, a drag down tips its top towards you.
    pub fn orbit(&mut self, delta: Vec2, pivot: Point) {
        let at = self.to_screen(pivot);
        let per_pixel = 0.01;
        self.turn = (self.turn + f64::from(delta.x) * per_pixel).rem_euclid(std::f64::consts::TAU);
        self.tilt = (self.tilt - f64::from(delta.y) * per_pixel).clamp(0.0, MAX_TILT_RAD);
        self.keep_at(pivot, at);
    }

    /// Set the orientation, keeping `pivot` where it is on screen.
    pub fn orient(&mut self, turn: f64, tilt: f64, pivot: Point) {
        let at = self.to_screen(pivot);
        self.turn = turn.rem_euclid(std::f64::consts::TAU);
        self.tilt = tilt.clamp(0.0, MAX_TILT_RAD);
        self.keep_at(pivot, at);
    }

    /// Move the offset so the stock-top point `world` lands on `screen`.
    pub fn keep_at(&mut self, world: Point, screen: Pos2) {
        let (across, up) = self.camera((world.0, world.1, self.top_z));
        self.offset = (
            f64::from(screen.x) - across * self.scale,
            f64::from(screen.y) + up * self.scale,
        );
    }
}
