# Chipload

Cross-platform 2.5D CAM application (DXF/DWG in, G-code out).

See `CLAUDE.md` for architecture rules, conventions, and workflow.

## Quick start

```
uv sync --all-extras        # or: pip install -e ".[dev,ui]"
pytest
chipload --help
```

Install the GUI extra with `pip install -e ".[ui]"`.

## Usage (v0)

```
chipload inspect part.dxf                        # units, entities, contours, issues
chipload run part.dxf --job job.json -o part.nc  # generate checked G-code
```

The viewer (`chipload-gui [part.dxf] [job.json]`) shows the drawing, stock, and generated
toolpath (start points numbered in cut order, arrows in the cutting direction, rapids dashed)
and saves G-code only after the check passes. Jobs are edited as JSON for now: click contours
in the viewer (Ctrl+click for more), then **Copy as selector** (Ctrl+Shift+C) and paste the
`{"handles": [...]}` into an operation's `select`. The Selection panel shows what that selector
would cut.

Supported: **profile** operations (inside, outside, on) with optional holding tabs
(`"tabs": {"count": 4, "width": 8, "height": 3}`, width = bridge left on the part), and
**pocket** operations with islands; climb or conventional, multiple depth passes, one tool
per job, and the GRBL post.
G-code is written only if it passes the independent checker. See
`tests/corpus/jobs/v3/` for example jobs and `docs/specs/` for exact behaviour and
limitations.
