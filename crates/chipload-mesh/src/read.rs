// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading an STL file's bytes (spec §2): binary when its length is exactly what its count
//! asks for, text otherwise when it starts with "solid".

use std::collections::HashMap;

use chipload_core::tolerances::MESH_DEGENERATE_SHARE;

use crate::{
    Format, MAX_BYTES, MAX_TRIANGLES, MAX_TRIANGLES_COUNTED, Mesh, MeshError, ReadReport, Vertex,
};

const HEADER: usize = 80;
const COUNT_END: usize = HEADER + 4;
const PER_TRIANGLE: usize = 50;

/// The triangles of an STL file, those without an area dropped, and what was found.
pub fn read_stl(bytes: &[u8]) -> Result<(Mesh, ReadReport), MeshError> {
    if bytes.len() > MAX_BYTES {
        return Err(MeshError::TooLarge(bytes.len()));
    }
    let (raw, format) = match binary_count(bytes) {
        Some(count) if bytes.len() == COUNT_END + PER_TRIANGLE * count => {
            (binary(bytes, count)?, Format::Binary)
        }
        count if starts_with_solid(bytes) => match text(bytes) {
            // Nothing read as text from a file with a binary count: a broken binary one.
            Ok(raw) if raw.is_empty() && count.is_some() => return Err(not_stl(bytes, count)),
            Ok(raw) => (raw, Format::Text),
            // A binary file whose header begins with "solid", with a few bytes too many or
            // too few, reads as broken text: say what the binary count asked for.
            Err(MeshError::Text(_)) if count.is_some() => return Err(not_stl(bytes, count)),
            Err(e) => return Err(e),
        },
        count => return Err(not_stl(bytes, count)),
    };
    let total = raw.len();
    let triangles: Vec<[Vertex; 3]> = raw.into_iter().filter(has_area).collect();
    if triangles.is_empty() {
        return Err(MeshError::Empty);
    }
    let open_edges = (triangles.len() <= MAX_TRIANGLES_COUNTED).then(|| open_edges(&triangles));
    let report = ReadReport {
        format,
        triangles: triangles.len(),
        dropped: total - triangles.len(),
        open_edges,
    };
    Ok((Mesh { triangles }, report))
}

fn not_stl(bytes: &[u8], count: Option<usize>) -> MeshError {
    MeshError::NotStl {
        expected: count.map_or(COUNT_END, |n| {
            COUNT_END.saturating_add(PER_TRIANGLE.saturating_mul(n))
        }),
        found: bytes.len(),
    }
}

/// The count a binary file states, if it is long enough to have one.
fn binary_count(bytes: &[u8]) -> Option<usize> {
    let count = bytes.get(HEADER..COUNT_END)?;
    let count = u32::from_le_bytes([count[0], count[1], count[2], count[3]]);
    usize::try_from(count).ok()
}

fn starts_with_solid(bytes: &[u8]) -> bool {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes[start..].starts_with(b"solid")
}

fn binary(bytes: &[u8], count: usize) -> Result<Vec<[Vertex; 3]>, MeshError> {
    if count > MAX_TRIANGLES {
        return Err(MeshError::TooManyTriangles(count));
    }
    let mut triangles = Vec::with_capacity(count);
    for record in bytes[COUNT_END..].as_chunks::<PER_TRIANGLE>().0 {
        // 12 bytes of normal, then three corners of three f32, then a 2-byte attribute.
        let mut corners = [[0.0; 3]; 3];
        for (k, corner) in corners.iter_mut().enumerate() {
            for (axis, value) in corner.iter_mut().enumerate() {
                let at = 12 + 12 * k + 4 * axis;
                let b = &record[at..at + 4];
                let v = f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                if !v.is_finite() {
                    return Err(MeshError::NotANumber);
                }
                *value = v;
            }
        }
        triangles.push(corners);
    }
    Ok(triangles)
}

/// The text form: every `vertex x y z` between `outer loop` and `endloop`, three to a
/// facet. Other words (names, normals, `endsolid`) are passed over.
fn text(bytes: &[u8]) -> Result<Vec<[Vertex; 3]>, MeshError> {
    let source = String::from_utf8_lossy(bytes);
    let mut triangles = Vec::new();
    let mut corners: Vec<Vertex> = Vec::with_capacity(3);
    let mut in_loop = false;
    for (n, line) in source.lines().enumerate() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("outer") => {
                if in_loop {
                    return Err(MeshError::Text(n + 1));
                }
                in_loop = true;
                corners.clear();
            }
            Some("vertex") => {
                if !in_loop || corners.len() == 3 {
                    return Err(MeshError::Text(n + 1));
                }
                let mut corner = [0.0; 3];
                for value in &mut corner {
                    let word = words.next().ok_or(MeshError::Text(n + 1))?;
                    let v: f64 = word.parse().map_err(|_| MeshError::Text(n + 1))?;
                    if !v.is_finite() {
                        return Err(MeshError::NotANumber);
                    }
                    *value = v;
                }
                corners.push(corner);
            }
            Some("endloop") => {
                if !in_loop || corners.len() != 3 {
                    return Err(MeshError::Text(n + 1));
                }
                in_loop = false;
                triangles.push([corners[0], corners[1], corners[2]]);
                if triangles.len() > MAX_TRIANGLES {
                    return Err(MeshError::TooManyTriangles(triangles.len()));
                }
            }
            _ => {}
        }
    }
    if in_loop {
        return Err(MeshError::Text(source.lines().count()));
    }
    Ok(triangles)
}

fn sub(a: Vertex, b: Vertex) -> Vertex {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm2(v: Vertex) -> f64 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

/// Whether a triangle has an area: twice its area squared against its longest edge to
/// the fourth power, so the test holds in any unit.
fn has_area([a, b, c]: &[Vertex; 3]) -> bool {
    let (u, v) = (sub(*b, *a), sub(*c, *a));
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let longest = norm2(u).max(norm2(v)).max(norm2(sub(*c, *b)));
    longest > 0.0 && norm2(cross) > (MESH_DEGENERATE_SHARE * longest).powi(2)
}

/// Edges used by one triangle only. Corners are matched exactly, as the file writes them
/// once per triangle; only the count leaves here, so the map's order doesn't matter.
fn open_edges(triangles: &[[Vertex; 3]]) -> usize {
    let mut ids: HashMap<[u64; 3], u32> = HashMap::new();
    let mut edges: Vec<(u32, u32)> = Vec::with_capacity(3 * triangles.len());
    for t in triangles {
        let mut corner = |v: Vertex| {
            let key = [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()];
            let next = u32::try_from(ids.len()).unwrap_or(u32::MAX);
            *ids.entry(key).or_insert(next)
        };
        let (a, b, c) = (corner(t[0]), corner(t[1]), corner(t[2]));
        for (p, q) in [(a, b), (b, c), (c, a)] {
            edges.push((p.min(q), p.max(q)));
        }
    }
    edges.sort_unstable();
    let mut open = 0;
    let mut k = 0;
    while k < edges.len() {
        let mut end = k + 1;
        while end < edges.len() && edges[end] == edges[k] {
            end += 1;
        }
        if end - k == 1 {
            open += 1;
        }
        k = end;
    }
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tetrahedron: closed, four triangles.
    pub(crate) fn tetrahedron() -> Vec<[Vertex; 3]> {
        let (a, b, c, d) = (
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
        );
        vec![[a, c, b], [a, b, d], [a, d, c], [b, c, d]]
    }

    pub(crate) fn binary_of(triangles: &[[Vertex; 3]], header: &[u8]) -> Vec<u8> {
        let mut out = vec![b' '; HEADER];
        out[..header.len()].copy_from_slice(header);
        out.extend((triangles.len() as u32).to_le_bytes());
        for t in triangles {
            out.extend([0u8; 12]);
            for v in t {
                for x in v {
                    out.extend((*x as f32).to_le_bytes());
                }
            }
            out.extend([0u8; 2]);
        }
        out
    }

    fn text_of(triangles: &[[Vertex; 3]]) -> String {
        let mut s = String::from("solid test\n");
        for t in triangles {
            s.push_str("  facet normal 0 0 0\n    outer loop\n");
            for v in t {
                s.push_str(&format!("      vertex {} {} {}\n", v[0], v[1], v[2]));
            }
            s.push_str("    endloop\n  endfacet\n");
        }
        s.push_str("endsolid test\n");
        s
    }

    #[test]
    fn binary_and_text_read_the_same_triangles() {
        let t = tetrahedron();
        let (bin, report) = read_stl(&binary_of(&t, b"made by a test")).unwrap();
        assert_eq!(report.format, Format::Binary);
        assert_eq!(bin.triangles, t);
        assert_eq!(report.open_edges, Some(0));
        let (txt, report) = read_stl(text_of(&t).as_bytes()).unwrap();
        assert_eq!(report.format, Format::Text);
        assert_eq!(txt.triangles, t);
    }

    #[test]
    fn a_binary_file_whose_header_says_solid_is_still_binary() {
        let t = tetrahedron();
        let (mesh, report) = read_stl(&binary_of(&t, b"solid but binary")).unwrap();
        assert_eq!((report.format, mesh.triangles.len()), (Format::Binary, 4));
        // One byte short: neither form, and the error says what the count asked for.
        let mut short = binary_of(&t, b"solid but binary");
        short.pop();
        assert_eq!(
            read_stl(&short).unwrap_err(),
            MeshError::NotStl {
                expected: 84 + 200,
                found: 283
            }
        );
    }

    #[test]
    fn triangles_without_area_are_dropped_and_gaps_counted() {
        let mut t = tetrahedron();
        t.pop(); // a gap: three edges now open
        t.push([[1.0, 1.0, 1.0], [2.0, 2.0, 2.0], [3.0, 3.0, 3.0]]); // on one line
        t.push([[5.0, 5.0, 5.0]; 3]); // a point
        let (mesh, report) = read_stl(&binary_of(&t, b"")).unwrap();
        assert_eq!((mesh.triangles.len(), report.dropped), (3, 2));
        assert_eq!(report.open_edges, Some(3));
    }

    #[test]
    fn hostile_input_is_refused() {
        assert!(matches!(read_stl(b""), Err(MeshError::NotStl { .. })));
        assert!(matches!(read_stl(b"hello"), Err(MeshError::NotStl { .. })));
        assert_eq!(read_stl(b"solid x\nendsolid x\n"), Err(MeshError::Empty));
        let nan = "solid\nfacet normal 0 0 0\nouter loop\nvertex 0 0 NaN\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid\n";
        assert_eq!(read_stl(nan.as_bytes()), Err(MeshError::NotANumber));
        let two = "solid\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nendloop\nendsolid\n";
        assert_eq!(read_stl(two.as_bytes()), Err(MeshError::Text(5)));
        let mut bin = binary_of(&tetrahedron(), b"");
        bin[12 + COUNT_END..16 + COUNT_END].copy_from_slice(&f32::INFINITY.to_le_bytes());
        assert_eq!(read_stl(&bin), Err(MeshError::NotANumber));
        // A count that asks for more than there is.
        let mut lying = binary_of(&tetrahedron(), b"");
        lying[HEADER..COUNT_END].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read_stl(&lying), Err(MeshError::NotStl { .. })));
    }

    proptest::proptest! {
        /// Any bytes: an error or triangles, never a panic.
        #[test]
        fn any_bytes_read_without_panicking(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..600)) {
            let _ = read_stl(&bytes);
        }
    }
}
