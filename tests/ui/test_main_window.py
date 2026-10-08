"""GUI smoke tests: the window opens files, generates, draws and saves through core."""

from pathlib import Path

import pytest

from chipload.core.units import Units
from chipload.ui.main_window import MainWindow

CORPUS = Path(__file__).parent.parent / "corpus"
SIMPLE = CORPUS / "dxf" / "simple_part.dxf"
MESSY = CORPUS / "dxf" / "messy.dxf"
JOB = CORPUS / "jobs" / "v2" / "simple_part_full.json"


@pytest.fixture
def window(qtbot):
    w = MainWindow()
    qtbot.addWidget(w)
    return w


def test_starts_empty(window):
    assert not window.generate_action.isEnabled()
    assert not window.save_action.isEnabled()


def test_open_dxf_draws_contours(window):
    assert window.open_dxf(SIMPLE)
    assert window.view.item_count("contours") == 4
    assert "Contours (4)" in window.messages.toPlainText()


def test_unitless_dxf_asks_for_units(window):
    asked = []

    def answer():
        asked.append(True)
        return Units.MM

    assert window.open_dxf(MESSY, ask_units=answer)
    assert asked == [True]
    assert "units_from_job" not in window.messages.toPlainText()
    assert "Units: mm (from job)" in window.messages.toPlainText()


def test_unitless_dxf_is_not_opened_if_user_cancels(window):
    assert not window.open_dxf(MESSY, ask_units=lambda: None)
    assert window.view.item_count("contours") == 0


def test_generate_and_save(window, tmp_path):
    assert window.open_job(JOB)
    assert window.view.item_count("stock") == 1
    assert window.open_dxf(SIMPLE)
    assert window.generate_action.isEnabled()
    assert window.generate()
    assert window.view.item_count("path") > 0
    assert window.save_action.isEnabled()
    assert "Check passed" in window.messages.toPlainText()

    out = tmp_path / "part.nc"
    assert window.save_gcode(out)
    assert out.read_text() == (CORPUS / "expected" / "v2_simple_part_full.nc").read_text()


def test_changing_inputs_discards_the_old_result(window):
    window.open_job(JOB)
    window.open_dxf(SIMPLE)
    window.generate()
    window.open_dxf(SIMPLE)
    assert window.result is None
    assert not window.save_action.isEnabled()
    assert window.view.item_count("path") == 0


def test_rejected_program_is_shown_but_cannot_be_saved(window, tmp_path):
    job = tmp_path / "job.json"
    job.write_text(JOB.read_text().replace('"x": 800', '"x": 100'))
    window.open_job(job)
    window.open_dxf(SIMPLE)
    assert not window.generate()
    assert window.view.item_count("path") > 0  # the rejected path is drawn for diagnosis
    assert not window.save_action.isEnabled()
    assert "check FAILED" in window.messages.toPlainText()


def test_generation_error_is_reported(window, tmp_path):
    job = tmp_path / "job.json"
    job.write_text(JOB.read_text().replace('"depth": 12,', '"depth": 30,', 1))
    window.open_job(job)
    window.open_dxf(SIMPLE)
    assert not window.generate()
    assert "deeper than the stock" in window.messages.toPlainText()


def test_bad_job_file_is_reported(window, tmp_path):
    job = tmp_path / "job.json"
    job.write_text("{nope")
    assert not window.open_job(job)
    assert "Cannot open job" in window.messages.toPlainText()


def test_tabs_are_drawn(window):
    window.open_job(CORPUS / "jobs" / "v3" / "simple_part_tabs.json")
    window.open_dxf(SIMPLE)
    assert window.generate()
    assert window.view.item_count("tabs") == 4
    window.open_job(JOB)
    assert window.view.item_count("tabs") == 0  # inputs changed: old toolpath and tabs gone
    window.generate()
    assert window.view.item_count("tabs") == 0
