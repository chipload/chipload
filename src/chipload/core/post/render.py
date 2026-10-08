"""Render a Program to G-code text through a post-processor template.

Only generic work happens here: number formatting and modal compression. Anything machine
specific belongs in the template.
"""

from __future__ import annotations

from dataclasses import dataclass

from jinja2 import PackageLoader, StrictUndefined, TemplateError
from jinja2.sandbox import SandboxedEnvironment

from chipload.core.errors import PostConfigError
from chipload.core.job.post_config import PostConfig
from chipload.core.toolpath.model import Program


@dataclass(frozen=True, slots=True)
class PostMove:
    """One move with pre-formatted words. None means "unchanged, omit"."""

    code: str
    x: str | None
    y: str | None
    z: str | None
    f: str | None


def format_number(value: float, decimals: int) -> str:
    text = f"{value:.{decimals}f}"
    if text.startswith("-") and float(text) == 0:
        return text[1:]  # never write -0.000
    return text


def render_program(program: Program, config: PostConfig) -> str:
    def fmt(value: float) -> str:
        return format_number(value, config.decimals)

    context = {
        "post_name": config.name,
        "tool_id": program.tool_id,
        "tool_diameter": fmt(program.tool_diameter),
        "rpm": format_number(program.rpm, 0),
        "spindle": program.spindle,
        "safe_z": fmt(program.safe_z),
        "dwell": fmt(config.spindle_dwell_s) if config.spindle_dwell_s > 0 else None,
        "moves": compress_moves(program, config.decimals),
    }
    environment = SandboxedEnvironment(
        loader=PackageLoader("chipload.core.post", "templates"),
        undefined=StrictUndefined,
        trim_blocks=True,
        lstrip_blocks=True,
        keep_trailing_newline=True,
        autoescape=False,
    )
    try:
        text = environment.get_template(config.template).render(context)
    except TemplateError as exc:
        raise PostConfigError(f"Template {config.template!r} failed: {exc}") from exc
    return text if text.endswith("\n") else text + "\n"


def compress_moves(program: Program, decimals: int) -> list[PostMove]:
    """Omit axis and feed words that have not changed; drop moves that go nowhere."""
    last: dict[str, str | None] = {"x": None, "y": None, "z": None, "f": None}
    result: list[PostMove] = []
    for move in program.moves:
        words: dict[str, str | None] = {}
        for axis, value in (("x", move.x), ("y", move.y), ("z", move.z)):
            text = None if value is None else format_number(value, decimals)
            words[axis] = text if text is not None and text != last[axis] else None
        if not any(words.values()):
            continue
        feed = None
        if move.kind == "feed":
            if move.feed is None:
                raise PostConfigError("Feed move without a feed rate")
            text = format_number(move.feed, decimals)
            feed = text if text != last["f"] else None
            last["f"] = text
        for axis in ("x", "y", "z"):
            if words[axis] is not None:
                last[axis] = words[axis]
        code = "G0" if move.kind == "rapid" else "G1"
        result.append(PostMove(code, words["x"], words["y"], words["z"], feed))
    return result
