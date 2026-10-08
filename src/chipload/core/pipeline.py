"""End to end: job + DXF -> checked G-code. The CLI and (later) the UI both call this."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from pathlib import Path

from chipload.core.errors import GcodeCheckFailed, JobError
from chipload.core.geometry.chain import Contour
from chipload.core.geometry.primitives import Point
from chipload.core.io.dxf_import import ImportReport, import_dxf
from chipload.core.job.models import Job, ProfileOp, Tool
from chipload.core.job.post_config import PostConfig, load_builtin_post
from chipload.core.ops.profile_op import plan_profile
from chipload.core.ops.select import select_contours
from chipload.core.post.render import render_program
from chipload.core.sim.gcode_check import CheckIssue, check_gcode, limits_from_job
from chipload.core.toolpath.model import Move, Program
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
    tool = _validate_job(job)
    rpm = tool.rpm
    if rpm is None:  # _validate_job already refuses this
        raise JobError(f"Tool {tool.id} has no spindle speed (rpm)")
    post = load_builtin_post(job.machine.post)
    imported = import_dxf(dxf_path, Units(job.source.units))

    top_z = 0.0 if job.stock.z_zero == "top" else job.stock.thickness
    moves: list[Move] = []
    notes: list[str] = []
    position: Point = (0.0, 0.0)
    for index, op in enumerate(job.operations, start=1):
        if not isinstance(op, ProfileOp):  # _validate_job already refuses these
            raise JobError(f"Operation {index}: '{op.type}' is not implemented in this version")
        label = f"Operation {index}" + (f" ({op.name})" if op.name else "")
        contours = select_contours(imported.contours, op.select, label)
        plan = plan_profile(op, label, contours, tool.diameter / 2, job.machine.spindle, position)
        notes.extend(plan.notes)
        pass_z = [top_z - d for d in pass_depths(op.depth, op.step_down)]
        for ring in plan.rings:
            moves.extend(
                contour_moves(
                    list(ring.points), pass_z, job.machine.safe_z, op.feed, op.plunge_feed
                )
            )
            position = ring.points[0]

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


def _validate_job(job: Job) -> Tool:
    """Generation-time rules from spec section 4. Returns the single tool used."""
    if not job.operations:
        raise JobError("Job has no operations")
    for index, op in enumerate(job.operations, start=1):
        if not isinstance(op, ProfileOp):
            raise JobError(f"Operation {index}: '{op.type}' is not implemented in this version")
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
    for index, op in enumerate(job.operations, start=1):
        if not isinstance(op, ProfileOp):
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
    return tool


def _extent_notes(job: Job, report: ImportReport) -> list[str]:
    if report.bbox is None:
        return []
    (x0, y0), (x1, y1) = report.bbox
    lo, hi = job.stock.xy_bounds()
    if x0 < lo[0] or y0 < lo[1] or x1 > hi[0] or y1 > hi[1]:
        return ["The drawing extends outside the stock"]
    return []
