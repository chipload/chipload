from __future__ import annotations

import argparse
import sys


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="chipload", description="2.5D CAM")
    sub = parser.add_subparsers(dest="command")

    inspect = sub.add_parser("inspect", help="Dump entities, units and issues of a DXF")
    inspect.add_argument("dxf")

    run = sub.add_parser("run", help="Generate G-code from a DXF and a job file")
    run.add_argument("dxf")
    run.add_argument("--job", required=True)
    run.add_argument("-o", "--output", required=True)

    args = parser.parse_args(argv)
    if args.command is None:
        parser.print_help()
        return 0
    print(f"'{args.command}' is not implemented yet", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
