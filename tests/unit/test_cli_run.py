from pathlib import Path

from chipload.cli.main import EXIT_FAILED, EXIT_OK, main

CORPUS = Path(__file__).parent.parent / "corpus"


def test_run_writes_checked_gcode(tmp_path, capsys):
    out = tmp_path / "part.nc"
    code = main([
        "run", str(CORPUS / "dxf" / "simple_part.dxf"),
        "--job", str(CORPUS / "jobs" / "v2" / "simple_part_full.json"),
        "-o", str(out),
    ])  # fmt: skip
    assert code == EXIT_OK
    expected = (CORPUS / "expected" / "v2_simple_part_full.nc").read_text()
    assert out.read_text() == expected
    assert not (tmp_path / "part.nc.tmp").exists()
    assert "Wrote" in capsys.readouterr().out


def test_failed_run_writes_nothing(tmp_path, capsys):
    out = tmp_path / "part.nc"
    job = tmp_path / "job.json"
    text = (CORPUS / "jobs" / "v2" / "simple_part_full.json").read_text()
    job.write_text(text.replace('"depth": 12,', '"depth": 30,', 1))
    code = main(["run", str(CORPUS / "dxf" / "simple_part.dxf"), "--job", str(job), "-o", str(out)])
    assert code == EXIT_FAILED
    assert not out.exists()
    assert "deeper than the stock" in capsys.readouterr().err


def test_newer_job_version_is_refused(tmp_path, capsys):
    job = tmp_path / "job.json"
    job.write_text('{"schema_version": 99}')
    code = main(["run", "x.dxf", "--job", str(job), "-o", str(tmp_path / "o.nc")])
    assert code == EXIT_FAILED
    assert "schema_version 99" in capsys.readouterr().err


def test_checker_failure_writes_nothing_and_keeps_old_file(tmp_path, capsys):
    out = tmp_path / "part.nc"
    out.write_text("previous program\n")
    job = tmp_path / "job.json"
    text = (CORPUS / "jobs" / "v2" / "simple_part_full.json").read_text()
    job.write_text(text.replace('"x": 800', '"x": 100'))
    code = main(["run", str(CORPUS / "dxf" / "simple_part.dxf"), "--job", str(job), "-o", str(out)])
    assert code == EXIT_FAILED
    assert out.read_text() == "previous program\n"
    err = capsys.readouterr().err
    assert "exceeds machine travel" in err
    assert "No output written" in err
