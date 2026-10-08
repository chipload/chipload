"""Human-readable rendering of core results for the terminal."""

from __future__ import annotations

from chipload.core.geometry.chain import Contour
from chipload.core.io.dxf_import import ImportReport


def format_import_report(report: ImportReport) -> list[str]:
    lines = [f"Units: {report.units.value} (from {report.units_from})"]
    if report.bbox is not None:
        (x0, y0), (x1, y1) = report.bbox
        lines.append(
            f"Extents: X {x0:.3f} .. {x1:.3f}, Y {y0:.3f} .. {y1:.3f} "
            f"({x1 - x0:.3f} x {y1 - y0:.3f} mm)"
        )
    for title, counts in (
        ("Imported", report.imported),
        ("Flattened", report.flattened),
        ("Ignored", report.ignored),
    ):
        if counts:
            lines.append(f"{title}: " + ", ".join(f"{kind} {n}" for kind, n in counts))
    if report.issues:
        lines.append(f"Issues ({len(report.issues)}):")
        lines.extend(f"  [{issue.severity}] {issue.message}" for issue in report.issues)
    else:
        lines.append("Issues: none")
    return lines


def format_contours(contours: tuple[Contour, ...]) -> list[str]:
    lines = [f"Contours ({len(contours)}):"]
    for c in contours:
        if not c.closed:
            state = "open"
        elif c.invalid_reason is not None:
            state = "INVALID"
        else:
            state = "outer" if (c.nesting_depth or 0) % 2 == 0 else "hole"
        pts = c.points()
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        lines.append(
            f"  #{c.id:<4} {state:<8} layer {', '.join(c.layers):<16} "
            f"X {min(xs):.3f} .. {max(xs):.3f}  Y {min(ys):.3f} .. {max(ys):.3f}"
        )
    return lines
