# gncam

Cross-platform 2.5D CAM application (DXF/DWG in, G-code out). Placeholder name: rename the package, CLI, and docs as needed.

See `CLAUDE.md` for architecture rules, conventions, and workflow.

## Quick start

```
python -m venv .venv && source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -e ".[dev]"
pytest
gncam --help
```

Install the GUI extra with `pip install -e ".[ui]"`.
