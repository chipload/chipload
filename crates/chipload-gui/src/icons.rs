// SPDX-License-Identifier: GPL-3.0-or-later
//! Small line icons painted with egui's painter, for the Browser and the header
//! (docs/specs/browser-and-layout.md §2, decision 8). Painted, not taken from an icon font,
//! so no font file or licence comes with them.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// Every icon the interface paints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Project,
    Settings,
    Tools,
    Endmill,
    Drill,
    Vbit,
    Drawing,
    Dxf,
    Layer,
    Image,
    Text,
    Circle,
    Rectangle,
    Polyline,
    Arc,
    Slot,
    Polygon,
    Offset,
    Paths,
    Profile,
    Pocket,
    Vcarve,
    FollowPath,
    Holes,
    Surface,
    Eye,
    EyeOff,
    New,
    Open,
    Save,
    Import,
    Undo,
    Redo,
    Help,
    Generate,
    Stop,
    Export,
    Fit,
    Top,
    ThreeD,
    Simulation,
    Measure,
    Grid,
    Toolpath,
    Part,
    Parts,
    Machine,
    ThemeLight,
    ThemeDark,
    ThemeSystem,
    /// Books on a shelf: the user's library.
    Library,
}

/// Line width of the icons, points.
const LINE: f32 = 1.2;
/// Segments of a painted circle.
const CIRCLE_STEPS: usize = 24;

/// Paint `icon` into `rect` (square is best) in `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let pen = Pen {
        painter,
        rect,
        stroke: Stroke::new(LINE, color),
    };
    match icon {
        Icon::Project => pen.cube(),
        Icon::Library => {
            pen.closed(&[(0.14, 0.2), (0.3, 0.2), (0.3, 0.82), (0.14, 0.82)]);
            pen.closed(&[(0.36, 0.3), (0.52, 0.3), (0.52, 0.82), (0.36, 0.82)]);
            pen.closed(&[(0.58, 0.28), (0.72, 0.24), (0.88, 0.78), (0.74, 0.82)]);
            pen.line([(0.08, 0.88), (0.92, 0.88)]);
        }
        Icon::Settings => pen.gear(),
        Icon::Tools => {
            pen.endmill(0.3);
            pen.line([(0.62, 0.15), (0.62, 0.85)]);
            pen.line([(0.62, 0.85), (0.78, 0.85)]);
        }
        Icon::Endmill => pen.endmill(0.5),
        Icon::Drill => {
            pen.poly(&[
                (0.38, 0.1),
                (0.38, 0.7),
                (0.5, 0.92),
                (0.62, 0.7),
                (0.62, 0.1),
            ]);
            pen.line([(0.38, 0.45), (0.62, 0.3)]);
            pen.line([(0.38, 0.65), (0.62, 0.5)]);
        }
        Icon::Vbit => {
            pen.poly(&[
                (0.4, 0.1),
                (0.4, 0.45),
                (0.25, 0.45),
                (0.5, 0.92),
                (0.75, 0.45),
            ]);
            pen.poly(&[(0.75, 0.45), (0.6, 0.45), (0.6, 0.1)]);
        }
        Icon::Drawing | Icon::Dxf => {
            pen.sheet();
            if icon == Icon::Dxf {
                pen.line([(0.3, 0.5), (0.7, 0.5)]);
                pen.line([(0.3, 0.68), (0.7, 0.68)]);
            } else {
                pen.circle((0.5, 0.6), 0.16);
            }
        }
        Icon::Layer => {
            pen.closed(&[(0.5, 0.2), (0.9, 0.4), (0.5, 0.6), (0.1, 0.4)]);
            pen.poly(&[(0.1, 0.6), (0.5, 0.8), (0.9, 0.6)]);
        }
        Icon::Image => {
            pen.closed(&[(0.1, 0.2), (0.9, 0.2), (0.9, 0.8), (0.1, 0.8)]);
            pen.poly(&[
                (0.15, 0.75),
                (0.4, 0.45),
                (0.58, 0.62),
                (0.7, 0.52),
                (0.85, 0.7),
            ]);
            pen.circle((0.68, 0.36), 0.06);
        }
        // A capital A (docs/specs/text.md §5).
        Icon::Text => {
            pen.poly(&[(0.18, 0.86), (0.5, 0.14), (0.82, 0.86)]);
            pen.poly(&[(0.31, 0.6), (0.69, 0.6)]);
        }
        Icon::Circle => pen.circle((0.5, 0.5), 0.36),
        Icon::Rectangle => pen.closed(&[(0.12, 0.25), (0.88, 0.25), (0.88, 0.75), (0.12, 0.75)]),
        Icon::Polyline => pen.poly(&[(0.1, 0.8), (0.3, 0.3), (0.55, 0.65), (0.9, 0.2)]),
        Icon::Arc => {
            // Screen y grows downwards: angles past PI bow upwards.
            let points = pen.arc((0.5, 0.78), 0.4, PI + 0.35, TAU - 0.35);
            pen.poly(&points);
            if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                pen.dot(first, 0.05);
                pen.dot(last, 0.05);
            }
        }
        Icon::Offset => {
            pen.closed(&[(0.35, 0.35), (0.65, 0.35), (0.65, 0.65), (0.35, 0.65)]);
            pen.rounded((0.5, 0.5), 0.38, 0.12);
        }
        Icon::Polygon => {
            let points: Vec<(f32, f32)> = (0..6)
                .map(|k| {
                    let t = k as f32 * TAU / 6.0;
                    (0.5 + 0.38 * t.cos(), 0.5 + 0.38 * t.sin())
                })
                .collect();
            pen.closed(&points);
        }
        Icon::Slot => {
            let mut points = pen.arc((0.68, 0.5), 0.2, -FRAC_PI_2, FRAC_PI_2);
            points.extend(pen.arc((0.32, 0.5), 0.2, FRAC_PI_2, PI + FRAC_PI_2));
            pen.closed(&points);
        }
        Icon::Paths => pen.poly(&[
            (0.1, 0.6),
            (0.25, 0.35),
            (0.4, 0.35),
            (0.6, 0.65),
            (0.75, 0.65),
            (0.9, 0.4),
        ]),
        Icon::Profile => {
            pen.closed(&[(0.3, 0.3), (0.7, 0.3), (0.7, 0.7), (0.3, 0.7)]);
            pen.rounded((0.5, 0.5), 0.35, 0.12);
        }
        Icon::Pocket => {
            pen.closed(&[(0.12, 0.12), (0.88, 0.12), (0.88, 0.88), (0.12, 0.88)]);
            pen.closed(&[(0.3, 0.3), (0.7, 0.3), (0.7, 0.7), (0.3, 0.7)]);
            pen.line([(0.45, 0.5), (0.55, 0.5)]);
        }
        Icon::Vcarve => {
            pen.poly(&[(0.1, 0.25), (0.5, 0.85), (0.9, 0.25)]);
            pen.line([(0.1, 0.25), (0.9, 0.25)]);
        }
        Icon::FollowPath => {
            pen.wave();
            pen.dot((0.12, 0.62), 0.07);
        }
        Icon::Holes => {
            pen.circle((0.5, 0.5), 0.34);
            pen.line([(0.5, 0.28), (0.5, 0.72)]);
            pen.line([(0.28, 0.5), (0.72, 0.5)]);
        }
        Icon::Surface => {
            pen.poly(&[
                (0.12, 0.25),
                (0.88, 0.25),
                (0.88, 0.5),
                (0.12, 0.5),
                (0.12, 0.75),
                (0.88, 0.75),
            ]);
        }
        Icon::New => {
            pen.sheet();
            pen.line([(0.36, 0.6), (0.66, 0.6)]);
            pen.line([(0.51, 0.45), (0.51, 0.75)]);
        }
        Icon::Open => {
            pen.closed(&[
                (0.08, 0.25),
                (0.38, 0.25),
                (0.46, 0.35),
                (0.92, 0.35),
                (0.92, 0.82),
                (0.08, 0.82),
            ]);
            pen.line([(0.08, 0.48), (0.92, 0.48)]);
        }
        Icon::Save => {
            pen.closed(&[
                (0.12, 0.12),
                (0.75, 0.12),
                (0.88, 0.25),
                (0.88, 0.88),
                (0.12, 0.88),
            ]);
            pen.closed(&[(0.3, 0.12), (0.66, 0.12), (0.66, 0.36), (0.3, 0.36)]);
            pen.closed(&[(0.28, 0.56), (0.72, 0.56), (0.72, 0.88), (0.28, 0.88)]);
        }
        Icon::Import => {
            pen.sheet();
            pen.line([(0.51, 0.38), (0.51, 0.78)]);
            pen.poly(&[(0.36, 0.63), (0.51, 0.78), (0.66, 0.63)]);
        }
        Icon::Undo | Icon::Redo => {
            let flip = |p: (f32, f32)| {
                if icon == Icon::Redo {
                    (1.0 - p.0, p.1)
                } else {
                    p
                }
            };
            // Over the top from right to left, the head at the left end.
            let arc: Vec<(f32, f32)> = pen
                .arc((0.5, 0.62), 0.3, 0.0, -PI)
                .into_iter()
                .map(flip)
                .collect();
            pen.poly(&arc);
            pen.poly(&[flip((0.06, 0.5)), flip((0.2, 0.66)), flip((0.34, 0.5))]);
        }
        Icon::Help => {
            pen.circle((0.5, 0.5), 0.4);
            let hook = pen.arc((0.5, 0.4), 0.14, -PI, FRAC_PI_2);
            pen.poly(&hook);
            pen.line([(0.5, 0.54), (0.5, 0.62)]);
            pen.dot((0.5, 0.74), 0.05);
        }
        Icon::Generate => {
            let points = vec![
                pen.at((0.25, 0.15)),
                pen.at((0.85, 0.5)),
                pen.at((0.25, 0.85)),
            ];
            painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
        }
        Icon::Stop => {
            let r = Rect::from_min_max(pen.at((0.22, 0.22)), pen.at((0.78, 0.78)));
            painter.rect_filled(r, 1.0, color);
        }
        Icon::Export => {
            pen.line([(0.5, 0.1), (0.5, 0.62)]);
            pen.poly(&[(0.3, 0.42), (0.5, 0.62), (0.7, 0.42)]);
            pen.poly(&[(0.12, 0.6), (0.12, 0.88), (0.88, 0.88), (0.88, 0.6)]);
        }
        Icon::Fit => {
            for (x, y, dx, dy) in [
                (0.1, 0.1, 1.0, 1.0),
                (0.9, 0.1, -1.0, 1.0),
                (0.9, 0.9, -1.0, -1.0),
                (0.1, 0.9, 1.0, -1.0),
            ] {
                pen.poly(&[(x, y + 0.25 * dy), (x, y), (x + 0.25 * dx, y)]);
            }
            pen.closed(&[(0.35, 0.35), (0.65, 0.35), (0.65, 0.65), (0.35, 0.65)]);
        }
        Icon::Top => {
            pen.closed(&[(0.15, 0.15), (0.85, 0.15), (0.85, 0.85), (0.15, 0.85)]);
            pen.line([(0.15, 0.5), (0.85, 0.5)]);
            pen.line([(0.5, 0.15), (0.5, 0.85)]);
        }
        Icon::ThreeD => pen.cube(),
        Icon::Simulation => {
            pen.poly(&[(0.08, 0.4), (0.08, 0.85), (0.92, 0.85), (0.92, 0.4)]);
            pen.poly(&[
                (0.08, 0.4),
                (0.3, 0.4),
                (0.3, 0.62),
                (0.62, 0.62),
                (0.62, 0.4),
                (0.92, 0.4),
            ]);
            pen.closed(&[(0.42, 0.08), (0.5, 0.08), (0.5, 0.5), (0.42, 0.5)]);
        }
        Icon::Measure => {
            pen.closed(&[(0.06, 0.62), (0.62, 0.06), (0.94, 0.38), (0.38, 0.94)]);
            for t in [0.25_f32, 0.4, 0.55, 0.7] {
                let a = (0.06 + 0.56 * t, 0.62 - 0.56 * t);
                pen.line([a, (a.0 + 0.12, a.1 + 0.12)]);
            }
        }
        Icon::Machine => {
            // A gantry over a bed: two uprights, the beam, the spindle down to the bed.
            pen.line([(0.08, 0.86), (0.92, 0.86)]);
            pen.poly(&[(0.18, 0.86), (0.18, 0.2), (0.82, 0.2), (0.82, 0.86)]);
            pen.closed(&[(0.42, 0.2), (0.58, 0.2), (0.58, 0.5), (0.42, 0.5)]);
            pen.line([(0.5, 0.5), (0.5, 0.66)]);
        }
        Icon::ThemeLight => {
            pen.circle((0.5, 0.5), 0.18);
            for k in 0..8 {
                let a = k as f32 * PI / 4.0;
                let (s, c) = a.sin_cos();
                pen.line([
                    (0.5 + 0.28 * c, 0.5 + 0.28 * s),
                    (0.5 + 0.42 * c, 0.5 + 0.42 * s),
                ]);
            }
        }
        Icon::ThemeDark => {
            // A crescent: the outer circle's arc, and an inner arc cutting it.
            let outer = pen.arc((0.5, 0.5), 0.38, 0.9, 0.9 + 1.55 * PI);
            pen.poly(&outer);
            let inner = pen.arc((0.66, 0.36), 0.3, 0.9 + 1.55 * PI - 0.95, 0.9 + 0.35);
            pen.poly(&inner);
        }
        Icon::ThemeSystem => {
            pen.circle((0.5, 0.5), 0.36);
            let half: Vec<Pos2> = pen
                .arc((0.5, 0.5), 0.36, -FRAC_PI_2, FRAC_PI_2)
                .into_iter()
                .map(|p| pen.at(p))
                .collect();
            painter.add(Shape::convex_polygon(half, color, Stroke::NONE));
        }
        Icon::Part => {
            pen.closed(&[
                (0.12, 0.22),
                (0.88, 0.22),
                (0.88, 0.78),
                (0.55, 0.78),
                (0.55, 0.6),
                (0.45, 0.6),
                (0.45, 0.78),
                (0.12, 0.78),
            ]);
            pen.circle((0.3, 0.45), 0.08);
            pen.circle((0.7, 0.45), 0.08);
        }
        Icon::Parts => {
            pen.closed(&[(0.25, 0.1), (0.9, 0.1), (0.9, 0.55), (0.25, 0.55)]);
            pen.poly(&[
                (0.25, 0.3),
                (0.1, 0.3),
                (0.1, 0.9),
                (0.75, 0.9),
                (0.75, 0.55),
            ]);
            pen.circle((0.57, 0.32), 0.07);
        }
        Icon::Grid => {
            for t in [0.15, 0.38, 0.62, 0.85] {
                pen.line([(t, 0.1), (t, 0.9)]);
                pen.line([(0.1, t), (0.9, t)]);
            }
        }
        Icon::Toolpath => {
            pen.poly(&[
                (0.1, 0.25),
                (0.75, 0.25),
                (0.75, 0.5),
                (0.25, 0.5),
                (0.25, 0.75),
                (0.9, 0.75),
            ]);
            pen.dot((0.1, 0.25), 0.06);
        }
        Icon::Eye | Icon::EyeOff => {
            pen.eye();
            if icon == Icon::EyeOff {
                pen.line([(0.15, 0.85), (0.85, 0.15)]);
            }
        }
    }
}

/// Paint the open (pointing down) or closed (pointing right) triangle of a folder.
pub fn chevron(painter: &Painter, rect: Rect, open: bool, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.22;
    let points = if open {
        vec![
            pos2(c.x - r, c.y - r * 0.6),
            pos2(c.x + r, c.y - r * 0.6),
            pos2(c.x, c.y + r * 0.8),
        ]
    } else {
        vec![
            pos2(c.x - r * 0.6, c.y - r),
            pos2(c.x + r * 0.8, c.y),
            pos2(c.x - r * 0.6, c.y + r),
        ]
    };
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

/// Paint a tick box: ticked or empty.
pub fn tick_box(painter: &Painter, rect: Rect, ticked: bool, color: Color32, fill: Color32) {
    let side = rect.width().min(rect.height()) * 0.72;
    let square = Rect::from_center_size(rect.center(), eframe::egui::vec2(side, side));
    if ticked {
        painter.rect_filled(square, 2.0, fill);
    }
    painter.rect_stroke(
        square,
        2.0,
        Stroke::new(LINE, color),
        eframe::egui::StrokeKind::Inside,
    );
    if ticked {
        let at = |x: f32, y: f32| square.min + eframe::egui::vec2(x * side, y * side);
        painter.add(Shape::line(
            vec![at(0.22, 0.52), at(0.42, 0.72), at(0.78, 0.3)],
            Stroke::new(LINE * 1.4, color),
        ));
    }
}

/// The tool's orange in Chipload's mark (packaging/brand/README.md).
pub const LOGO_TOOL: Color32 = Color32::from_rgb(0xf2, 0x9e, 0x38);

/// Chipload's mark (packaging/brand/make-brand.py): a C of three rings spiralling in, as a
/// pocket's rings are cut, and the tool, orange, where the cut starts.
pub fn paint_logo(painter: &Painter, rect: Rect, color: Color32) {
    let side = rect.width().min(rect.height());
    let pen = Pen {
        painter,
        rect,
        // As drawn on a 256 square: rings 20 wide at radii 100, 70 and 40.
        stroke: Stroke::new(side * 20.0 / 256.0, color),
    };
    // The opening faces right: from 42° round to 318°, counter-clockwise (Y is down here).
    let (start, end) = (-42f32.to_radians(), -318f32.to_radians());
    let mut points = Vec::new();
    for (i, r) in [100.0f32, 70.0, 40.0].into_iter().enumerate() {
        let (from, to) = if i % 2 == 0 {
            (start, end)
        } else {
            (end, start)
        };
        points.extend(pen.arc((0.5, 0.5), r / 256.0, from, to));
    }
    pen.poly(&points);
    let tool = (
        0.5 + 100.0 / 256.0 * start.cos(),
        0.5 + 100.0 / 256.0 * start.sin(),
    );
    painter.circle_filled(pen.at(tool), side * 23.0 / 256.0, LOGO_TOOL);
}

/// Lines in unit coordinates of the icon's square (0,0 top left, 1,1 bottom right).
struct Pen<'a> {
    painter: &'a Painter,
    rect: Rect,
    stroke: Stroke,
}

impl Pen<'_> {
    fn at(&self, (x, y): (f32, f32)) -> Pos2 {
        let side = self.rect.width().min(self.rect.height());
        let left = self.rect.center().x - side / 2.0;
        let top = self.rect.center().y - side / 2.0;
        pos2(left + x * side, top + y * side)
    }

    fn line(&self, ends: [(f32, f32); 2]) {
        self.painter
            .line_segment([self.at(ends[0]), self.at(ends[1])], self.stroke);
    }

    fn poly(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|&p| self.at(p)).collect();
        self.painter.add(Shape::line(points, self.stroke));
    }

    fn closed(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|&p| self.at(p)).collect();
        self.painter.add(Shape::closed_line(points, self.stroke));
    }

    fn arc(&self, centre: (f32, f32), r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
        (0..=CIRCLE_STEPS)
            .map(|i| {
                let t = from + (to - from) * i as f32 / CIRCLE_STEPS as f32;
                (centre.0 + r * t.cos(), centre.1 + r * t.sin())
            })
            .collect()
    }

    fn circle(&self, centre: (f32, f32), r: f32) {
        let points = self.arc(centre, r, 0.0, TAU);
        self.closed(&points[..CIRCLE_STEPS]);
    }

    fn dot(&self, centre: (f32, f32), r: f32) {
        let side = self.rect.width().min(self.rect.height());
        self.painter
            .circle_filled(self.at(centre), r * side, self.stroke.color);
    }

    /// A square of half-side `half` with corners rounded by `r`.
    fn rounded(&self, (cx, cy): (f32, f32), half: f32, r: f32) {
        let inner = half - r;
        let mut points = Vec::new();
        for (k, (sx, sy)) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
            .into_iter()
            .enumerate()
        {
            let start = k as f32 * FRAC_PI_2;
            points.extend(self.arc(
                (cx + sx * inner, cy + sy * inner),
                r,
                start,
                start + FRAC_PI_2,
            ));
        }
        self.closed(&points);
    }

    fn cube(&self) {
        let top = (0.5, 0.12);
        let left = (0.14, 0.31);
        let right = (0.86, 0.31);
        let middle = (0.5, 0.5);
        self.closed(&[top, right, middle, left]);
        self.poly(&[left, (0.14, 0.7), (0.5, 0.9), (0.86, 0.7), right]);
        self.line([middle, (0.5, 0.9)]);
    }

    fn gear(&self) {
        self.circle((0.5, 0.5), 0.24);
        self.circle((0.5, 0.5), 0.08);
        for k in 0..8 {
            let a = k as f32 * PI / 4.0;
            let (s, c) = a.sin_cos();
            self.line([
                (0.5 + 0.26 * c, 0.5 + 0.26 * s),
                (0.5 + 0.4 * c, 0.5 + 0.4 * s),
            ]);
        }
    }

    /// An endmill centred at `x`: a shank, and flutes below.
    fn endmill(&self, x: f32) {
        let w = 0.13;
        self.closed(&[(x - w, 0.1), (x + w, 0.1), (x + w, 0.9), (x - w, 0.9)]);
        self.line([(x - w, 0.5), (x + w, 0.38)]);
        self.line([(x - w, 0.68), (x + w, 0.56)]);
        self.line([(x - w, 0.86), (x + w, 0.74)]);
    }

    /// A sheet of paper with its top right corner folded.
    fn sheet(&self) {
        self.closed(&[
            (0.2, 0.08),
            (0.62, 0.08),
            (0.82, 0.28),
            (0.82, 0.92),
            (0.2, 0.92),
        ]);
        self.poly(&[(0.62, 0.08), (0.62, 0.28), (0.82, 0.28)]);
    }

    fn wave(&self) {
        let points: Vec<(f32, f32)> = (0..=CIRCLE_STEPS)
            .map(|i| {
                let t = i as f32 / CIRCLE_STEPS as f32;
                (0.12 + 0.76 * t, 0.5 - 0.12 * (t * TAU).sin())
            })
            .collect();
        self.poly(&points);
    }

    fn eye(&self) {
        let mut upper = self.arc((0.5, 0.95), 0.62, -2.35, -0.79);
        let lower = self.arc((0.5, 0.05), 0.62, 0.79, 2.35);
        upper.extend(lower);
        self.closed(&upper);
        self.circle((0.5, 0.5), 0.13);
    }
}
