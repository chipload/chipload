"""DXF import: read -> flatten blocks -> Line/Arc primitives -> chain -> classify -> nest -> report.

All input is treated as hostile. Nothing is dropped without an entry in the ImportReport.
"""

from __future__ import annotations

import math
from collections import Counter
from collections.abc import Iterable
from dataclasses import dataclass, replace
from itertools import pairwise
from pathlib import Path
from typing import Literal

import shapely
from ezdxf.document import Drawing
from ezdxf.entities.arc import Arc as DxfArc
from ezdxf.entities.circle import Circle as DxfCircle
from ezdxf.entities.dxfgfx import DXFGraphic
from ezdxf.entities.ellipse import Ellipse as DxfEllipse
from ezdxf.entities.insert import Insert as DxfInsert
from ezdxf.entities.line import Line as DxfLine
from ezdxf.entities.lwpolyline import LWPolyline as DxfLWPolyline
from ezdxf.entities.polyline import Polyline as DxfPolyline
from ezdxf.entities.spline import Spline as DxfSpline
from ezdxf.filemanagement import readfile
from ezdxf.lldxf.const import DXFError
from ezdxf.math import OCS, Vec3, bulge_to_arc
from shapely.geometry import Polygon

from chipload.core.errors import DxfImportError, DxfUnitsMissing
from chipload.core.geometry.chain import Contour, Segment, chain_segments
from chipload.core.geometry.primitives import Arc, Line, Point, Primitive
from chipload.core.tolerances import (
    ANGLE_TOL_RAD,
    ARC_APPROX_TOL_MM,
    CHAIN_TOL_MM,
    Z_NONZERO_TOL_MM,
)
from chipload.core.units import Units, to_mm

# $INSUNITS codes accepted in v0. Anything else is refused rather than guessed.
_INSUNITS = {1: Units.INCH, 4: Units.MM}
_UNITLESS = 0

# Guards against hostile, self-referencing or absurdly deep block nesting.
MAX_BLOCK_DEPTH = 32

Severity = Literal["warning", "error"]


@dataclass(frozen=True, slots=True)
class Issue:
    severity: Severity
    code: str
    message: str
    location: Point | None = None


@dataclass(frozen=True, slots=True)
class ImportReport:
    units: Units
    units_from: Literal["dxf", "job"]
    imported: tuple[tuple[str, int], ...]  # (DXF type, count), sorted by type
    flattened: tuple[tuple[str, int], ...]  # splines/ellipses approximated by lines
    ignored: tuple[tuple[str, int], ...]  # entity types not machined
    issues: tuple[Issue, ...]
    bbox: tuple[Point, Point] | None  # (min, max) of all imported geometry, mm


@dataclass(frozen=True, slots=True)
class ImportResult:
    contours: tuple[Contour, ...]
    report: ImportReport


def import_dxf(path: str | Path, units: Units | None) -> ImportResult:
    """Import a DXF file.

    `units` normally comes from the job and must agree with $INSUNITS if the file declares it.
    None means "use the file's units"; a unitless file is then refused.
    """
    try:
        doc = readfile(path)
    except (OSError, DXFError, UnicodeError) as exc:
        raise DxfImportError(f"Cannot read DXF {path}: {exc}") from exc
    return import_document(doc, units)


def import_document(doc: Drawing, units: Units | None) -> ImportResult:
    file_units, units_from = _resolve_units(doc, units)
    converter = _Converter(scale=to_mm(1.0, file_units))
    for entity in doc.modelspace():
        converter.convert(entity, root_handle=entity.dxf.handle, parent_layer=None, depth=0)

    issues = list(converter.issues)
    if units_from == "job":
        issues.append(
            Issue(
                "warning",
                "units_from_job",
                f"DXF has no units; using job units ({file_units.value})",
            )
        )
    if converter.nonzero_z:
        issues.append(
            Issue(
                "warning",
                "nonzero_z",
                f"{converter.nonzero_z} entities have nonzero Z (max |Z| "
                f"{converter.max_abs_z:.3f} mm); Z was dropped",
            )
        )

    chained = chain_segments(converter.segments)
    for seg in chained.zero_length:
        issues.append(
            Issue("warning", "zero_length", f"Removed zero-length {_describe(seg)}", seg.prim.start)
        )
    for seg in chained.duplicates:
        issues.append(
            Issue("warning", "duplicate", f"Removed duplicate {_describe(seg)}", seg.prim.start)
        )
    for point in chained.branch_points:
        issues.append(
            Issue("warning", "branch_point", "More than two segments meet at one point", point)
        )

    contours = _classify(chained.contours, issues)
    report = ImportReport(
        units=file_units,
        units_from=units_from,
        imported=_sorted_counts(converter.imported),
        flattened=_sorted_counts(converter.flattened),
        ignored=_sorted_counts(converter.ignored),
        issues=tuple(issues),
        bbox=_bbox(converter.segments),
    )
    return ImportResult(contours, report)


def _resolve_units(doc: Drawing, job_units: Units | None) -> tuple[Units, Literal["dxf", "job"]]:
    code = doc.header.get("$INSUNITS", _UNITLESS)
    if code == _UNITLESS:
        if job_units is None:
            raise DxfUnitsMissing("DXF has no units ($INSUNITS=0); specify mm or inch explicitly")
        return job_units, "job"
    if code not in _INSUNITS:
        raise DxfImportError(
            f"DXF $INSUNITS={code} is not supported (only mm, inch or unitless). "
            "Re-export the drawing in mm or inch."
        )
    dxf_units = _INSUNITS[code]
    if job_units is not None and dxf_units != job_units:
        raise DxfImportError(
            f"DXF declares {dxf_units.value} but the job says {job_units.value}. "
            "Fix the job or the drawing; units are never guessed."
        )
    return dxf_units, "dxf"


class _Converter:
    """Turns DXF entities into Segments in mm, collecting counts and issues."""

    def __init__(self, scale: float) -> None:
        self.scale = scale
        self.segments: list[Segment] = []
        self.issues: list[Issue] = []
        self.imported: Counter[str] = Counter()
        self.flattened: Counter[str] = Counter()
        self.ignored: Counter[str] = Counter()
        self.nonzero_z = 0
        self.max_abs_z = 0.0

    def convert(
        self, entity: DXFGraphic, root_handle: str, parent_layer: str | None, depth: int
    ) -> None:
        kind = entity.dxftype()
        layer = str(entity.dxf.layer)
        if parent_layer is not None and layer == "0":
            layer = parent_layer  # block content on layer 0 takes the INSERT's layer

        if isinstance(entity, DxfInsert):
            self._insert(entity, root_handle, layer, depth)
            return
        converted = self._primitives(entity)
        if converted is None:
            return
        self.imported[kind] += 1
        self._track_z(max((abs(z) for _, z in converted), default=0.0))
        for prim, _ in converted:
            self.segments.append(Segment(self._scaled(prim), layer, root_handle, kind))

    def _primitives(self, entity: DXFGraphic) -> list[tuple[Primitive, float]] | None:
        """(primitive in drawing units, |z|) pairs, or None if the entity was not imported."""
        if isinstance(entity, DxfLine):
            return self._line(entity)
        if isinstance(entity, DxfArc | DxfCircle):  # Arc subclasses Circle; both handled here
            return self._arc(entity)
        if isinstance(entity, DxfLWPolyline):
            return self._lwpolyline(entity)
        if isinstance(entity, DxfPolyline):
            return self._polyline(entity)
        if isinstance(entity, DxfSpline | DxfEllipse):
            return self._flattened_curve(entity)
        self.ignored[entity.dxftype()] += 1
        return None

    def _line(self, entity: DxfLine) -> list[tuple[Primitive, float]]:
        start, end = Vec3(entity.dxf.start), Vec3(entity.dxf.end)
        return [(Line((start.x, start.y), (end.x, end.y)), max(abs(start.z), abs(end.z)))]

    def _arc(self, entity: DxfCircle) -> list[tuple[Primitive, float]] | None:
        ocs = self._planar_ocs(entity)
        if ocs is None:
            return None
        radius = float(entity.dxf.radius)
        if radius <= 0:
            self._skip(entity, f"radius {radius} is not positive")
            return None
        center = Vec3(entity.dxf.center)
        if isinstance(entity, DxfArc):
            start = math.radians(entity.dxf.start_angle)
            sweep = math.radians((entity.dxf.end_angle - entity.dxf.start_angle) % 360.0)
        else:
            start, sweep = 0.0, 2.0 * math.pi
        return [_ocs_arc_to_wcs(ocs, center, radius, start, sweep)]

    def _lwpolyline(self, entity: DxfLWPolyline) -> list[tuple[Primitive, float]] | None:
        ocs = self._planar_ocs(entity)
        if ocs is None:
            return None
        elevation = float(entity.dxf.elevation)
        vertices = [(float(x), float(y), float(b)) for x, y, b in entity.get_points("xyb")]
        return _bulge_polyline(ocs, vertices, elevation, entity.closed)

    def _polyline(self, entity: DxfPolyline) -> list[tuple[Primitive, float]] | None:
        if entity.is_3d_polyline:
            points = [Vec3(v.dxf.location) for v in entity.vertices]
            if entity.is_closed and points:
                points.append(points[0])
            return [
                (Line((a.x, a.y), (b.x, b.y)), max(abs(a.z), abs(b.z))) for a, b in pairwise(points)
            ]
        if not entity.is_2d_polyline:
            self.ignored["POLYLINE (mesh)"] += 1
            return None
        ocs = self._planar_ocs(entity)
        if ocs is None:
            return None
        elevation = float(Vec3(entity.dxf.elevation).z)
        vertices = [
            (float(v.dxf.location[0]), float(v.dxf.location[1]), float(v.dxf.bulge))
            for v in entity.vertices
        ]
        return _bulge_polyline(ocs, vertices, elevation, entity.is_closed)

    def _flattened_curve(self, entity: DxfSpline | DxfEllipse) -> list[tuple[Primitive, float]]:
        # Flatten in drawing units with the tolerance converted, so the error stays in mm.
        points = [Vec3(p) for p in entity.flattening(ARC_APPROX_TOL_MM / self.scale)]
        self.flattened[entity.dxftype()] += 1
        return [
            (Line((a.x, a.y), (b.x, b.y)), max(abs(a.z), abs(b.z))) for a, b in pairwise(points)
        ]

    def _insert(self, entity: DxfInsert, root_handle: str, layer: str, depth: int) -> None:
        if depth >= MAX_BLOCK_DEPTH:
            self.issues.append(
                Issue("error", "block_depth", f"Block nesting deeper than {MAX_BLOCK_DEPTH}")
            )
            return

        def skipped(child: DXFGraphic, reason: str) -> None:
            self.ignored[f"{child.dxftype()} (in block)"] += 1
            self.issues.append(
                Issue("warning", "block_child_skipped", f"{child.dxftype()} in block: {reason}")
            )

        inserts: Iterable[DxfInsert] = entity.multi_insert() if entity.mcount > 1 else [entity]
        for insert in inserts:
            for child in insert.virtual_entities(skipped_entity_callback=skipped):
                self.convert(child, root_handle, layer, depth + 1)

    # -- helpers

    def _planar_ocs(self, entity: DXFGraphic) -> OCS | None:
        normal = Vec3(entity.dxf.extrusion).normalize()
        if math.hypot(normal.x, normal.y) > ANGLE_TOL_RAD:
            self._skip(entity, "not parallel to the XY plane")
            return None
        return entity.ocs()

    def _skip(self, entity: DXFGraphic, reason: str) -> None:
        self.ignored[entity.dxftype()] += 1
        handle = entity.dxf.handle or "in block"
        self.issues.append(
            Issue("warning", "entity_skipped", f"{entity.dxftype()} ({handle}): {reason}")
        )

    def _track_z(self, abs_z: float) -> None:
        if abs_z * self.scale > Z_NONZERO_TOL_MM:
            self.nonzero_z += 1
            self.max_abs_z = max(self.max_abs_z, abs_z * self.scale)

    def _scaled(self, prim: Primitive) -> Primitive:
        s = self.scale
        if isinstance(prim, Line):
            return Line((prim.start[0] * s, prim.start[1] * s), (prim.end[0] * s, prim.end[1] * s))
        return Arc(
            (prim.center[0] * s, prim.center[1] * s), prim.radius * s, prim.start_angle, prim.sweep
        )


def _ocs_arc_to_wcs(
    ocs: OCS, center: Vec3, radius: float, start_angle: float, sweep: float
) -> tuple[Primitive, float]:
    """Map an OCS arc to WCS. A mirrored OCS (extrusion -Z) reverses the sweep direction."""
    c = ocs.to_wcs(center)
    s = ocs.to_wcs(
        Vec3(
            center.x + radius * math.cos(start_angle),
            center.y + radius * math.sin(start_angle),
            center.z,
        )
    )
    mirrored = Vec3(ocs.uz).z < 0
    wcs_start = math.atan2(s.y - c.y, s.x - c.x)
    return Arc((c.x, c.y), radius, wcs_start, -sweep if mirrored else sweep), abs(c.z)


def _bulge_polyline(
    ocs: OCS, vertices: list[tuple[float, float, float]], elevation: float, closed: bool
) -> list[tuple[Primitive, float]]:
    result: list[tuple[Primitive, float]] = []
    count = len(vertices) if closed else len(vertices) - 1
    for i in range(max(count, 0)):
        x1, y1, bulge = vertices[i]
        x2, y2, _ = vertices[(i + 1) % len(vertices)]
        if bulge == 0.0:
            a = ocs.to_wcs(Vec3(x1, y1, elevation))
            b = ocs.to_wcs(Vec3(x2, y2, elevation))
            result.append((Line((a.x, a.y), (b.x, b.y)), abs(a.z)))
            continue
        center, ccw_start, ccw_end, radius = bulge_to_arc((x1, y1), (x2, y2), bulge)
        ccw_sweep = (ccw_end - ccw_start) % (2.0 * math.pi)
        # bulge_to_arc always describes a CCW arc; a negative bulge runs it backwards from vertex i.
        start, sweep = (ccw_start, ccw_sweep) if bulge > 0 else (ccw_end, -ccw_sweep)
        result.append(
            _ocs_arc_to_wcs(ocs, Vec3(center.x, center.y, elevation), radius, start, sweep)
        )
    return result


def _classify(contours: tuple[Contour, ...], issues: list[Issue]) -> tuple[Contour, ...]:
    """Mark invalid closed contours, compute nesting depth, report open contours."""
    by_id = {c.id: c for c in contours}
    polygons: dict[int, Polygon] = {}
    classified: dict[int, Contour] = {}
    for contour in contours:
        if not contour.closed:
            issues.append(
                Issue(
                    "warning",
                    "open_contour",
                    f"Contour {contour.id} on {', '.join(contour.layers)} is open: "
                    f"ends at ({contour.start[0]:.3f}, {contour.start[1]:.3f}) and "
                    f"({contour.end[0]:.3f}, {contour.end[1]:.3f})",
                    contour.start,
                )
            )
            classified[contour.id] = contour
            continue
        reason = _invalid_reason(contour.points())
        if reason is not None:
            issues.append(
                Issue("error", "invalid_contour", f"Contour {contour.id}: {reason}", contour.start)
            )
            classified[contour.id] = replace(contour, invalid_reason=reason)
            continue
        polygons[contour.id] = Polygon(contour.points())

    for cid, polygon in polygons.items():
        depth = sum(
            1 for other_id, other in polygons.items() if other_id != cid and other.contains(polygon)
        )
        classified[cid] = replace(by_id[cid], nesting_depth=depth)
    return tuple(classified[c.id] for c in contours)


def _invalid_reason(ring: list[Point]) -> str | None:
    if len(ring) < 3:
        return "fewer than 3 distinct points"
    polygon = Polygon(ring)
    if not polygon.is_valid:
        return f"self-intersecting ({shapely.is_valid_reason(polygon)})"
    if polygon.area < CHAIN_TOL_MM * CHAIN_TOL_MM:
        return "encloses no area"
    return None


def _describe(segment: Segment) -> str:
    return f"{segment.source_type} on layer {segment.layer} ({segment.handle})"


def _sorted_counts(counter: Counter[str]) -> tuple[tuple[str, int], ...]:
    return tuple(sorted(counter.items()))


def _bbox(segments: list[Segment]) -> tuple[Point, Point] | None:
    points = [p for s in segments for p in s.prim.points()]
    if not points:
        return None
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    return (min(xs), min(ys)), (max(xs), max(ys))
