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

v0 supports closed-contour **profile** operations (inside, outside, on) with climb or
conventional milling and multiple depth passes, one tool per job, and the GRBL post.
G-code is written only if it passes the independent checker. See
`tests/corpus/jobs/v2/simple_part_full.json` for an example job and
`docs/specs/v0-profile-pipeline.md` for exact behaviour and limitations.
