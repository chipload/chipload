import pytest

from chipload.core.errors import PostConfigError, UnsupportedPostConfigVersion
from chipload.core.job.post_config import load_builtin_post, parse_post_config
from chipload.core.post.render import format_number, render_program
from chipload.core.sim.gcode_check import CheckLimits, check_gcode
from chipload.core.toolpath.model import Move, Program
from chipload.core.toolpath.profile import contour_moves

GRBL = load_builtin_post("grbl")


def square_program(spindle="cw"):
    ring = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]
    moves = contour_moves(ring, [-3.0, -6.0], safe_z=5.0, feed=1800.0, plunge_feed=600.0)
    return Program(1, 6.0, 18000.0, spindle, 5.0, tuple(moves))


EXPECTED_SQUARE = """\
(Chipload - GRBL)
(Tool 1: 6.000 mm)
G21 G90 G17 G94
M3 S18000
G4 P3.000
G0 Z5.000
G0 X0.000 Y0.000
G1 Z-3.000 F600.000
G1 Y10.000 F1800.000
G1 X10.000
G1 Y0.000
G1 X0.000
G1 Z-6.000 F600.000
G1 Y10.000 F1800.000
G1 X10.000
G1 Y0.000
G1 X0.000
G0 Z5.000
M5
M30
"""


def test_grbl_output_is_exact():
    assert render_program(square_program(), GRBL) == EXPECTED_SQUARE


def test_output_is_deterministic():
    assert render_program(square_program(), GRBL) == render_program(square_program(), GRBL)


def test_ccw_spindle_uses_m4():
    assert "M4 S18000" in render_program(square_program("ccw"), GRBL)


def test_rendered_program_passes_the_checker():
    limits = CheckLimits(5.0, 0.0, -18.0, (-5.0, -5.0), (20.0, 20.0), 3.0, (800.0, 600.0, 100.0))
    report = check_gcode(render_program(square_program(), GRBL), limits)
    assert report.ok, report.errors
    assert report.warnings == ()


def test_zero_dwell_is_omitted():
    config = GRBL.model_copy(update={"spindle_dwell_s": 0})
    assert "G4" not in render_program(square_program(), config)


def test_moves_that_go_nowhere_are_dropped():
    moves = (Move("rapid", z=5.0), Move("rapid", z=5.0), Move("rapid", x=1.0, y=1.0))
    text = render_program(Program(1, 6.0, 1000.0, "cw", 5.0, moves), GRBL)
    assert text.count("G0 Z5.000") == 1


@pytest.mark.parametrize(
    ("value", "decimals", "expected"),
    [(-0.0001, 3, "0.000"), (-0.0, 3, "0.000"), (1.23456, 3, "1.235"), (18000.0, 0, "18000")],
)
def test_format_number(value, decimals, expected):
    assert format_number(value, decimals) == expected


# -- config format


def valid_config(**changes):
    data = {
        "schema_version": 1,
        "name": "Test",
        "template": "grbl.nc.j2",
        "spindle_dwell_s": 0,
    }
    data.update(changes)
    return data


def test_config_missing_version_is_refused():
    data = valid_config()
    del data["schema_version"]
    with pytest.raises(UnsupportedPostConfigVersion):
        parse_post_config(data)


def test_config_newer_version_is_refused():
    with pytest.raises(UnsupportedPostConfigVersion):
        parse_post_config(valid_config(schema_version=99))


def test_config_template_path_traversal_is_refused():
    with pytest.raises(PostConfigError):
        parse_post_config(valid_config(template="../../etc/passwd"))


def test_config_dwell_is_required():
    data = valid_config()
    del data["spindle_dwell_s"]
    with pytest.raises(PostConfigError):
        parse_post_config(data)


def test_unknown_builtin_post():
    with pytest.raises(PostConfigError, match="Unknown"):
        load_builtin_post("haas")


def test_builtin_post_name_cannot_escape():
    with pytest.raises(PostConfigError):
        load_builtin_post("../posts/grbl")
