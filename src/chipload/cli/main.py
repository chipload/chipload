from __future__ import annotations

import argparse
import sys

from chipload.cli.report import format_contours, format_import_report
from chipload.core.errors import ChiploadError
from chipload.core.io.dxf_import import import_dxf
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
    print(f"'{args.command}' is not implemented yet", file=sys.stderr)
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


if __name__ == "__main__":
    raise SystemExit(main())
