from pathlib import Path

from chipload.cli.main import EXIT_FAILED, EXIT_OK, main

DXF = Path(__file__).parent.parent / "corpus" / "dxf"


def test_inspect_simple_part(capsys):
    assert main(["inspect", str(DXF / "simple_part.dxf")]) == EXIT_OK
    out = capsys.readouterr().out
    assert "Units: mm (from dxf)" in out
    assert "Contours (4):" in out


def test_inspect_unitless_file_needs_units(capsys):
    assert main(["inspect", str(DXF / "messy.dxf")]) == EXIT_FAILED
    assert "no units" in capsys.readouterr().err


def test_inspect_messy_file_reports_problems(capsys):
    assert main(["inspect", str(DXF / "messy.dxf"), "--units", "mm"]) == EXIT_OK
    out = capsys.readouterr().out
    for expected in ("duplicate", "zero-length", "is open", "nonzero Z", "Ignored: TEXT 1"):
        assert expected in out


def test_inspect_missing_file(capsys):
    assert main(["inspect", "does-not-exist.dxf"]) == EXIT_FAILED
    assert "Cannot read DXF" in capsys.readouterr().err
