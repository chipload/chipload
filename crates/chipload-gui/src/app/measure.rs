// SPDX-License-Identifier: GPL-3.0-or-later
//! View → Measure (docs/specs/small-items.md §3): two clicks, snapped as drawing is, give the
//! distance, ΔX, ΔY and the angle. Nothing goes into the project.

use super::*;

/// A measurement being made: the first point, then the second.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Measure {
    pub start: Option<Point>,
    pub end: Option<Point>,
}

impl Measure {
    /// A click: the second point, or (after a finished measurement) a new first one.
    pub fn click(&mut self, p: Point) {
        match (self.start, self.end) {
            (Some(_), None) => self.end = Some(p),
            _ => {
                self.start = Some(p);
                self.end = None;
            }
        }
    }
}

/// `L 26.926  ΔX 25.000  ΔY 10.000  21.8°` from `a` to `b`, in `units`.
pub fn measure_text(a: Point, b: Point, units: chipload_job::Units) -> String {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    let angle = if length > 0.0 {
        dy.atan2(dx).to_degrees()
    } else {
        0.0
    };
    let angle = if angle.abs() < 0.05 { 0.0 } else { angle };
    let n = |mm: f64| crate::draw::fixed(mm, units);
    format!("L {}  ΔX {}  ΔY {}  {angle:.1}°", n(length), n(dx), n(dy))
}

impl ChiploadApp {
    /// View → Measure (M): on, or off again.
    pub fn toggle_measure(&mut self) {
        if self.measure.is_some() {
            self.measure = None;
            return;
        }
        if self.three_d {
            self.view_top(); // measuring works in top view only
        }
        self.sketch = None;
        self.measure = Some(Measure::default());
    }

    /// A click on the canvas while measuring, at `p` (already snapped).
    pub fn measure_click(&mut self, p: Point) {
        if let Some(measure) = self.measure.as_mut() {
            measure.click(p);
        }
    }

    /// What the canvas shows: the measured line (to the cursor while the second point is
    /// still to come) and its text.
    pub(super) fn measure_scene(&self, cursor: Option<Point>) -> Option<(Point, Point, String)> {
        let measure = self.measure?;
        let start = measure.start?;
        let end = measure.end.or(cursor)?;
        Some((start, end, measure_text(start, end, self.units)))
    }

    /// The status bar while measuring.
    pub(super) fn measure_status(&self, snapped: &str) -> Option<String> {
        let measure = self.measure?;
        Some(match (measure.start, measure.end) {
            (None, _) => {
                format!("Measure: click the first point{snapped} (Alt: no snapping; Esc or M ends)")
            }
            (Some(_), None) => format!("Measure: click the second point{snapped}"),
            (Some(a), Some(b)) => format!(
                "{}  · click to measure again, Esc or M ends",
                measure_text(a, b, self.units)
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_clicks_measure_and_a_third_starts_again() {
        let mut m = Measure::default();
        m.click((0.0, 0.0));
        m.click((25.0, 10.0));
        assert_eq!(m.end, Some((25.0, 10.0)));
        assert_eq!(
            measure_text(m.start.unwrap(), m.end.unwrap(), chipload_job::Units::Mm),
            "L 26.926  ΔX 25.000  ΔY 10.000  21.8°"
        );
        m.click((5.0, 5.0));
        assert_eq!((m.start, m.end), (Some((5.0, 5.0)), None));
        assert_eq!(
            measure_text((10.0, 0.0), (0.0, -10.0), chipload_job::Units::Mm),
            "L 14.142  ΔX -10.000  ΔY -10.000  -135.0°"
        );
        assert_eq!(
            measure_text((1.0, 1.0), (1.0, 1.0), chipload_job::Units::Mm),
            "L 0.000  ΔX 0.000  ΔY 0.000  0.0°"
        );
        assert_eq!(
            measure_text((0.0, 0.0), (25.4, 0.0), chipload_job::Units::Inch),
            "L 1.0000  ΔX 1.0000  ΔY 0.0000  0.0°"
        );
    }
}
