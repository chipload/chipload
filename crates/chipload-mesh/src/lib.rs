// SPDX-License-Identifier: GPL-3.0-or-later
//! Triangle meshes for relief carving (docs/specs/stl-relief.md): an STL file's bytes in,
//! triangles out, turned so a chosen axis points up, and the shadow they cast seen from
//! above. No file I/O: callers pass the bytes.
//!
//! The reader is written from the format's public description: the binary form is an
//! 80-byte header, a little-endian u32 triangle count and 50 bytes per triangle (a normal,
//! three corners as f32, a 2-byte attribute); the text form is `solid … facet normal …
//! outer loop vertex x y z (three times) endloop endfacet … endsolid`. Stored normals are
//! ignored.

mod light;
mod read;
mod shadow;

use chipload_i18n::Message;

pub use light::LIGHT_TRIANGLES;
pub use read::read_stl;
pub use shadow::{Shadow, TopHeights};

/// More triangles than this is refused (spec §2).
pub const MAX_TRIANGLES: usize = 5_000_000;
/// A file larger than this is refused: a binary file of `MAX_TRIANGLES`, with room for
/// a text file of about a million triangles.
pub const MAX_BYTES: usize = 300 * 1024 * 1024;
/// Open edges are counted only up to this many triangles: counting needs memory in
/// proportion.
pub const MAX_TRIANGLES_COUNTED: usize = 2_000_000;

/// A corner: x, y, z in the file's units.
pub type Vertex = [f64; 3];

/// Triangles, three corners each.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub triangles: Vec<[Vertex; 3]>,
}

/// Which way an STL file was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Binary,
    Text,
}

/// What reading a file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadReport {
    pub format: Format,
    /// Triangles kept.
    pub triangles: usize,
    /// Triangles dropped as having no area.
    pub dropped: usize,
    /// Edges that belong to one triangle only: gaps in the surface. None when the mesh is
    /// too large to count ([`MAX_TRIANGLES_COUNTED`]).
    pub open_edges: Option<usize>,
}

/// Why a file can't be read. Its text is a message (`mesh.ftl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshError {
    /// Larger than [`MAX_BYTES`].
    TooLarge(usize),
    /// More triangles than [`MAX_TRIANGLES`].
    TooManyTriangles(usize),
    /// Neither a whole binary STL nor a text one: the size the count asks for and the
    /// size there is.
    NotStl { expected: usize, found: usize },
    /// A corner that isn't a finite number.
    NotANumber,
    /// The text form broke at this line (1-based).
    Text(usize),
    /// No triangles with an area.
    Empty,
}

impl MeshError {
    /// The error as a message.
    pub fn message(&self) -> Message {
        match self {
            MeshError::TooLarge(bytes) => Message::new("mesh-error-too-large").arg("bytes", *bytes),
            MeshError::TooManyTriangles(n) => {
                Message::new("mesh-error-too-many-triangles").arg("count", *n)
            }
            MeshError::NotStl { expected, found } => Message::new("mesh-error-not-stl")
                .arg("expected", *expected)
                .arg("found", *found),
            MeshError::NotANumber => Message::new("mesh-error-not-a-number"),
            MeshError::Text(line) => Message::new("mesh-error-text").arg("line", *line),
            MeshError::Empty => Message::new("mesh-error-empty"),
        }
    }
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message().english())
    }
}

impl std::error::Error for MeshError {}

/// Which of a model's axes points up when it lies on the board (spec §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Up {
    Z,
    Y,
    X,
    MinusZ,
    MinusY,
    MinusX,
}

impl Up {
    /// The corner turned so this axis points up: a turn, never a mirror, so the model keeps
    /// its handedness. Y up turns about X (Y to Z, Z to −Y); X up turns about Y (X to Z,
    /// Z to −X); the minus ones turn the other way.
    pub fn turn(self, [x, y, z]: Vertex) -> Vertex {
        match self {
            Up::Z => [x, y, z],
            Up::Y => [x, -z, y],
            Up::X => [-z, y, x],
            Up::MinusZ => [x, -y, -z],
            Up::MinusY => [x, z, -y],
            Up::MinusX => [z, y, -x],
        }
    }
}

impl Mesh {
    /// The lowest and highest corners, or None for no triangles.
    pub fn bounds(&self) -> Option<(Vertex, Vertex)> {
        let mut corners = self.triangles.iter().flatten();
        let first = *corners.next()?;
        Some(corners.fold((first, first), |(lo, hi), v| {
            (
                [lo[0].min(v[0]), lo[1].min(v[1]), lo[2].min(v[2])],
                [hi[0].max(v[0]), hi[1].max(v[1]), hi[2].max(v[2])],
            )
        }))
    }

    /// Every corner moved by `f`, the triangles kept in order.
    pub fn mapped(&self, f: impl Fn(Vertex) -> Vertex) -> Mesh {
        Mesh {
            triangles: self
                .triangles
                .iter()
                .map(|t| [f(t[0]), f(t[1]), f(t[2])])
                .collect(),
        }
    }

    /// Turned so `up` points up.
    pub fn turned(&self, up: Up) -> Mesh {
        self.mapped(|v| up.turn(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_up_turns_its_axis_to_z_and_keeps_handedness() {
        let axes = [
            (Up::Z, [0.0, 0.0, 1.0]),
            (Up::Y, [0.0, 1.0, 0.0]),
            (Up::X, [1.0, 0.0, 0.0]),
            (Up::MinusZ, [0.0, 0.0, -1.0]),
            (Up::MinusY, [0.0, -1.0, 0.0]),
            (Up::MinusX, [-1.0, 0.0, 0.0]),
        ];
        for (up, axis) in axes {
            assert_eq!(up.turn(axis), [0.0, 0.0, 1.0], "{up:?}");
            // A turn keeps x × y = z.
            let (x, y, z) = (
                up.turn([1.0, 0.0, 0.0]),
                up.turn([0.0, 1.0, 0.0]),
                up.turn([0.0, 0.0, 1.0]),
            );
            let cross = [
                x[1] * y[2] - x[2] * y[1],
                x[2] * y[0] - x[0] * y[2],
                x[0] * y[1] - x[1] * y[0],
            ];
            assert_eq!(cross, z, "{up:?}");
        }
    }
}
