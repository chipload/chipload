from __future__ import annotations

import argparse
import sys
from pathlib import Path

from chipload.cli.report import format_contours, format_import_report
from chipload.core.errors import ChiploadError, GcodeCheckFailed
from chipload.core.io.dxf_import import import_dxf
from chipload.core.io.output import write_text_atomically
from chipload.core.job.loader import load_job
from chipload.core.pipeline import run_job
from chipload.core.units import Units

EXIT_OK = 0
EXIT_FAILED = 1
EXIT_USAGE = 2


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="chipload", description="2.5D CAM")
    sub = parser.add_subparsers(dest="command")

    inspect = sub.add_parser("inspect", help="Dump entities, units and issues of a DXF")
    inspect.add_argument("dxf")
    inspect.add_argument(
        "--units",
        choices=[u.value for u in Units],
        help="Units of the drawing; required if the DXF does not declare them",
    )

    run = sub.add_parser("run", help="Generate G-code from a DXF and a job file")
    run.add_argument("dxf")
    run.add_argument("--job", required=True)
    run.add_argument("-o", "--output", required=True)

    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help()
        return EXIT_OK
    if args.command == "inspect":
        return _inspect(args.dxf, Units(args.units) if args.units else None)
    if args.command == "run":
        return _run(args.dxf, args.job, Path(args.output))
    parser.print_help()
    return EXIT_USAGE


def _inspect(path: str, units: Units | None) -> int:
    try:
        result = import_dxf(path, units)
    except ChiploadError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_FAILED
    for line in [*format_import_report(result.report), *format_contours(result.contours)]:
        print(line)
    return EXIT_OK


def _run(dxf: str, job_path: str, output: Path) -> int:
    try:
        job = load_job(job_path)
        result = run_job(job, dxf)
    except GcodeCheckFailed as exc:
        print(f"error: {exc}", file=sys.stderr)
        print("No output written.", file=sys.stderr)
        return EXIT_FAILED
    except ChiploadError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_FAILED

    for line in format_import_report(result.import_report):
        print(line)
    for note in result.notes:
        print(f"warning: {note}")
    for warning in result.check_warnings:
        print(f"check warning: {warning.message}")
    write_text_atomically(output, result.gcode)
    print(f"Wrote {output} ({result.gcode.count(chr(10))} lines, post {result.post.name})")
    return EXIT_OK


if __name__ == "__main__":
    raise SystemExit(main())
