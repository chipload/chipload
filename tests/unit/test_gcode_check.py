import pytest

from chipload.core.sim.gcode_check import CheckLimits, check_gcode

LIMITS = CheckLimits(
    safe_z=5.0,
    stock_top_z=0.0,
    stock_bottom_z=-18.0,
    stock_min=(0.0, 0.0),
    stock_max=(600.0, 400.0),
    tool_radius=3.0,
    travel=(800.0, 600.0, 100.0),
)

HEADER = "G21 G90 G17 G94\nG0 Z5\nM3 S18000\nG4 P3\n"
FOOTER = "G0 Z5\nM5\nM30\n"
GOOD_BODY = """\
G0 X10 Y10
G1 Z-6 F600
G1 X50 F1800
G1 Y50
G1 X10
G1 Y10
"""
GOOD = HEADER + GOOD_BODY + FOOTER


def errors(program: str, limits: CheckLimits = LIMITS) -> list[str]:
    return [i.message for i in check_gcode(program, limits).errors]


def test_good_program_passes():
    report = check_gcode(GOOD, LIMITS)
    assert report.ok, report.errors
    assert report.warnings == ()


def test_comments_are_ignored():
    assert errors("(T1 D=6.000)\n" + GOOD.replace("G1 Y50", "G1 Y50 ; side 2")) == []


@pytest.mark.parametrize(
    ("program", "expected"),
    [
        ("G21 G90\nG0 Z5\nG0 X10 Y10\nG1 Z-1 F100\n", "spindle off"),
        (HEADER.replace("G21 ", "") + GOOD_BODY, "before G21"),
        (HEADER.replace("G21", "G20") + GOOD_BODY, "G20"),
        (HEADER.replace(" G90", "") + GOOD_BODY, "before G90"),
        (HEADER + "G91\n", "G91"),
        (HEADER + "G0 X10 Y10\nG1 Z-1\n", "no feed"),
        (HEADER + "G0 X10 Y10\nG1 Z-1 F0\n", "must be positive"),
        (HEADER + "G0 X10 Y10\nG1 Z-1 F100\nG0 X20\n", "retract first"),
        ("G21 G90\nG0 X10 Y10\n", "retract first"),  # Z unknown
        (HEADER + "G0 X10 Y10\nG0 Z-1\n", "rapid into material"),
        (HEADER + "G0 X10 Y10\nG1 Z-18.5 F100\n", "below the stock bottom"),
        (HEADER + "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I4 J0\n", "radius mismatch"),
        (HEADER + "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10\n", "without I/J"),
        (HEADER + "G28\n", "G28 is not supported"),
        (HEADER + "M8\n", "M8 is not supported"),
        (HEADER + "G0 X10 Y10 X20\n", "twice"),
        (HEADER + "G0 G1 X10\n", "more than one motion"),
        (HEADER + "G0 X10 Y10 (unclosed\n", "comment"),
        (HEADER + "G0 X10 Y10 @\n", "Cannot parse"),
        (HEADER + "T1\n", "Unsupported word T"),
        (GOOD + "G0 Z10\n", "after program end"),
        (HEADER + "G0 X0 Y0\nG0 X900\n" + FOOTER, "exceeds machine travel"),
    ],
)
def test_bad_programs_are_flagged(program, expected):
    found = errors(program)
    assert any(expected.lower() in e.lower() for e in found), found


def test_valid_arc_passes():
    body = "G0 X10 Y10\nG1 Z-1 F100\nG2 X20 Y10 I5 J0\nG3 X10 Y10 I-5 J0\n"
    assert errors(HEADER + body + FOOTER) == []


def test_rapid_down_to_just_above_stock_is_allowed():
    assert errors(HEADER + "G0 X10 Y10\nG0 Z1\nG1 Z-1 F100\n" + FOOTER) == []


def test_cutting_outside_stock_is_a_warning():
    program = HEADER + "G0 X-20 Y10\nG1 Z-1 F100\nG1 X-30\n" + FOOTER
    report = check_gcode(program, LIMITS)
    assert report.ok
    assert any("outside the stock" in w.message for w in report.warnings)


def test_missing_travel_is_a_warning():
    limits = CheckLimits(5.0, 0.0, -18.0, (0.0, 0.0), (600.0, 400.0), 3.0, travel=None)
    report = check_gcode(GOOD, limits)
    assert report.ok
    assert any("travel not set" in w.message for w in report.warnings)


def test_missing_program_end_is_a_warning():
    report = check_gcode(HEADER + GOOD_BODY, LIMITS)
    assert any("M30" in w.message for w in report.warnings)
