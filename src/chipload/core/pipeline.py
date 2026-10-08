"""End to end: job + DXF -> checked G-code. The CLI and (later) the UI both call this."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from chipload.core.errors import GcodeCheckFailed, GeometryError, JobError, SelectorError
from chipload.core.geometry.chain import Contour
from chipload.core.geometry.primitives import Point
from chipload.core.io.dxf_import import ImportReport, import_dxf
from chipload.core.job.models import DrillOp, Job, PocketOp, ProfileOp, Tool
from chipload.core.job.post_config import PostConfig, load_builtin_post
from chipload.core.ops.profile_op import plan_profile
from chipload.core.ops.select import Selection, select_contours
from chipload.core.post.render import render_program
from chipload.core.sim.gcode_check import CheckIssue, check_gcode, limits_from_job
from chipload.core.toolpath.model import Move, Program
from chipload.core.toolpath.pocket import plan_pocket, pocket_moves
from chipload.core.toolpath.profile import contour_moves, pass_depths
from chipload.core.units import Units


@dataclass(frozen=True, slots=True)
class RunResult:
    gcode: str
    post: PostConfig
    import_report: ImportReport
    contours: tuple[Contour, ...]  # everything imported, for display
    program: Program  # the toolpath the G-code was rendered from
    notes: tuple[str, ...]  # toolpath warnings
    check_warnings: tuple[CheckIssue, ...]


def run_job(job: Job, dxf_path: str | Path) -> RunResult:
    """Generate G-code. Raises a ChiploadError subclass for anything that is not safe to cut,
    including a failed independent check, so unchecked G-code never leaves this function."""
    _verify_source(job, Path(dxf_path))
    tool, tool_notes = _validate_job(job)
    rpm = tool.rpm
    if rpm is None:  # _validate_job already refuses this
        raise JobError(f"Tool {tool.id} has no spindle speed (rpm)")
    post = load_builtin_post(job.machine.post)
    imported = import_dxf(dxf_path, Units(job.source.units))

    top_z = 0.0 if job.stock.z_zero == "top" else job.stock.thickness
    moves: list[Move] = []
    notes: list[str] = list(tool_notes)
    position: Point = (0.0, 0.0)
    for index, op in enumerate(job.operations, start=1):
        label = f"Operation {index}" + (f" ({op.name})" if op.name else "")
        if isinstance(op, DrillOp):  # _validate_job already refuses these
            raise JobError(f"{label}: 'drill' is not implemented in this version")
        selection = select_contours(imported.contours, op.select, label)
        notes.extend(selection.notes)
        pass_z = [top_z - d for d in pass_depths(op.depth, op.step_down)]
        if isinstance(op, ProfileOp):
            new_moves, new_notes, position = _profile(
                op, label, selection, tool, job, pass_z, position
            )
        else:
            new_moves, new_notes, position = _pocket(
                op, label, selection, tool, job, pass_z, position
            )
        moves.extend(new_moves)
        notes.extend(new_notes)

    program = Program(
        tool.id, tool.diameter, rpm, job.machine.spindle, job.machine.safe_z, tuple(moves)
    )
    gcode = render_program(program, post)
    report = check_gcode(gcode, limits_from_job(job))
    if not report.ok:
        raise GcodeCheckFailed([f"line {e.line}: {e.message}" for e in report.errors], program)
    notes.extend(_extent_notes(job, imported.report))
    return RunResult(
        gcode=gcode,
        post=post,
        import_report=imported.report,
        contours=imported.contours,
        program=program,
        notes=tuple(notes),
        check_warnings=report.warnings,
    )


Planned = tuple[list[Move], list[str], Point]


def _profile(
    op: ProfileOp,
    label: str,
    selection: Selection,
    tool: Tool,
    job: Job,
    pass_z: list[float],
    position: Point,
) -> Planned:
    plan = plan_profile(
        op, label, list(selection.contours), tool.diameter / 2, job.machine.spindle, position
    )
    moves: list[Move] = []
    for ring in plan.rings:
        moves += contour_moves(
            list(ring.points), pass_z, job.machine.safe_z, op.feed, op.plunge_feed
        )
        position = ring.points[0]
    return moves, list(plan.notes), position


def _pocket(
    op: PocketOp,
    label: str,
    selection: Selection,
    tool: Tool,
    job: Job,
    pass_z: list[float],
    position: Point,
) -> Planned:
    try:
        plan = plan_pocket(
            [(c.id, c.points()) for c in selection.contours],
            tool.diameter / 2,
            op.stepover * tool.diameter,
            op.direction,
            job.machine.spindle,
            op.keep_tool_down,
            position,
        )
    except (GeometryError, SelectorError) as exc:
        raise type(exc)(f"{label}: {exc}") from exc
    moves: list[Move] = []
    for piece in plan.pieces:
        moves += pocket_moves(piece, pass_z, job.machine.safe_z, op.feed, op.plunge_feed)
        position = piece.rings[-1][0]  # retracted above the start of the wall ring
    return moves, [f"{label}: {n}" for n in plan.notes], position


def _verify_source(job: Job, dxf_path: Path) -> None:
    if job.source.sha256 is None:
        return
    try:
        digest = hashlib.sha256(dxf_path.read_bytes()).hexdigest()
    except OSError as exc:
        raise JobError(f"Cannot read {dxf_path}: {exc}") from exc
    if digest.lower() != job.source.sha256.lower():
        raise JobError(
            f"{dxf_path} does not match the job's source.sha256; the drawing has changed"
        )


def _validate_job(job: Job) -> tuple[Tool, list[str]]:
    """Generation-time rules from the specs. Returns the single tool used and warnings."""
    if not job.operations:
        raise JobError("Job has no operations")
    for index, op in enumerate(job.operations, start=1):
        if isinstance(op, DrillOp):
            raise JobError(f"Operation {index}: 'drill' is not implemented in this version")
    used = sorted({op.tool for op in job.operations})
    if len(used) > 1:
        raise JobError(f"Operations use tools {used}; this version supports one tool per job")
    tools = {t.id: t for t in job.tools}
    if used[0] not in tools:
        raise JobError(f"Tool {used[0]} is not defined in the job")
    tool = tools[used[0]]
    if tool.rpm is None or tool.rpm <= 0:
        raise JobError(f"Tool {tool.id} has no spindle speed (rpm)")

    top = 0.0 if job.stock.z_zero == "top" else job.stock.thickness
    if job.machine.safe_z <= top:
        raise JobError(
            f"safe_z {job.machine.safe_z} must be above the stock top (Z {top}); "
            f"z_zero is '{job.stock.z_zero}'"
        )
    notes: list[str] = []
    for index, op in enumerate(job.operations, start=1):
        if isinstance(op, DrillOp):
            continue
        if op.depth > job.stock.thickness:
            raise JobError(
                f"Operation {index}: depth {op.depth} is deeper than the stock "
                f"({job.stock.thickness}); overcut is not supported yet"
            )
        max_feed = job.machine.max_feed
        for name, value in (("feed", op.feed), ("plunge_feed", op.plunge_feed)):
            if max_feed is not None and value > max_feed:
                raise JobError(
                    f"Operation {index}: {name} {value} exceeds machine max_feed {max_feed}"
                )
        if isinstance(op, PocketOp):
            notes += _pocket_tool_notes(index, tool)
    return tool, notes


def _pocket_tool_notes(index: int, tool: Tool) -> list[str]:
    """Pockets plunge straight down (no ramping yet), so they need a centre-cutting endmill."""
    if tool.type != "endmill":
        raise JobError(
            f"Operation {index}: pockets need an endmill, tool {tool.id} is a {tool.type}"
        )
    if tool.center_cutting is False:
        raise JobError(
            f"Operation {index}: tool {tool.id} is not centre-cutting and pockets plunge straight "
            "down; use a centre-cutting endmill"
        )
    if tool.center_cutting is None:
        note = (
            f"Operation {index}: pocket plunges straight down; tool {tool.id} isn't marked "
            "center_cutting, make sure it is a centre-cutting endmill"
        )
        return [note]
    return []


def _extent_notes(job: Job, report: ImportReport) -> list[str]:
    if report.bbox is None:
        return []
    (x0, y0), (x1, y1) = report.bbox
    lo, hi = job.stock.xy_bounds()
    if x0 < lo[0] or y0 < lo[1] or x1 > hi[0] or y1 > hi[1]:
        return ["The drawing extends outside the stock"]
    return []
