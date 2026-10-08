"""Click-to-select: picking contours by clicking near their line and copying a selector."""

import json
from pathlib import Path

import ezdxf
import pytest
from PySide6.QtCore import QPointF, Qt
from PySide6.QtGui import QGuiApplication

from chipload.ui.main_window import MainWindow

MESSY_2 = Path(__file__).parent.parent / "corpus" / "dxf" / "messy_2.dxf"

# Contours in messy_2.dxf (see tests/unit/test_corpus_messy_2.py).
CIRCLE_12 = (-25.0 + 3.6, -15.0)  # on the line of circle #12, handle 5A
CIRCLE_13 = (-49.0 + 3.6, -15.0)  # on the line of circle #13, handle 5B
CENTRELINE_22 = (64.75, 35.0)  # open centreline on layer upper, handle 64
EMPTY_INSIDE_FRAME = (0.0, -42.0)  # inside the outline, away from every line
OUTSIDE_EVERYTHING = (0.0, 75.0)


@pytest.fixture
def window(qtbot):
    w = MainWindow()
    qtbot.addWidget(w)
    w.resize(1200, 800)
    w.show()
    qtbot.waitExposed(w)
    assert w.open_dxf(MESSY_2)
    return w


def click(qtbot, window, point, modifier=Qt.KeyboardModifier.NoModifier):
    pos = window.view.mapFromScene(QPointF(*point))
    qtbot.mouseClick(window.view.viewport(), Qt.MouseButton.LeftButton, modifier, pos)


def selected_ids(window):
    return [c.id for c in window.view.selected_contours()]


def test_click_on_a_line_selects_that_contour(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    assert selected_ids(window) == [12]
    assert window.selector_json() == '{"handles": ["5A"]}'
    text = window.selection_text.toPlainText()
    assert "#12" in text
    assert "As a job selector it cuts: #12" in text


def test_click_inside_a_shape_away_from_lines_selects_nothing(qtbot, window):
    click(qtbot, window, EMPTY_INSIDE_FRAME)
    assert selected_ids(window) == []


def test_ctrl_click_adds_and_removes(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    click(qtbot, window, CIRCLE_13, Qt.KeyboardModifier.ControlModifier)
    assert selected_ids(window) == [12, 13]
    assert json.loads(window.selector_json()) == {"handles": ["5A", "5B"]}
    click(qtbot, window, CIRCLE_12, Qt.KeyboardModifier.ControlModifier)
    assert selected_ids(window) == [13]


def test_plain_click_on_another_contour_replaces_the_selection(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    click(qtbot, window, CIRCLE_13)
    assert selected_ids(window) == [13]


def test_click_on_empty_space_and_escape_clear(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    click(qtbot, window, OUTSIDE_EVERYTHING)
    assert selected_ids(window) == []
    click(qtbot, window, CIRCLE_12)
    qtbot.keyClick(window.view, Qt.Key.Key_Escape)
    assert selected_ids(window) == []
    assert not window.copy_selector_button.isEnabled()


def test_dragging_pans_without_losing_the_selection(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    view = window.view
    view.scale(4, 4)  # zoom in so there is room to scroll
    before = (view.horizontalScrollBar().value(), view.verticalScrollBar().value())
    start = view.mapFromScene(QPointF(*OUTSIDE_EVERYTHING))
    qtbot.mousePress(view.viewport(), Qt.MouseButton.LeftButton, pos=start)
    for step in range(1, 6):
        qtbot.mouseMove(view.viewport(), start + type(start)(step * 20, step * 10))
    qtbot.mouseRelease(view.viewport(), Qt.MouseButton.LeftButton, pos=start + type(start)(100, 50))
    after = (view.horizontalScrollBar().value(), view.verticalScrollBar().value())
    assert after != before
    assert selected_ids(window) == [12]


def test_copy_puts_the_selector_on_the_clipboard(qtbot, window):
    click(qtbot, window, CIRCLE_12)
    window.copy_selector()
    assert QGuiApplication.clipboard().text() == '{"handles": ["5A"]}'


def test_preview_explains_an_open_contour_selection(qtbot, window):
    click(qtbot, window, CENTRELINE_22)
    assert selected_ids(window) == [22]
    assert "only open contours" in window.selection_text.toPlainText()


def test_preview_warns_when_a_block_handle_selects_more(qtbot, tmp_path):
    doc = ezdxf.new()
    doc.header["$INSUNITS"] = 4
    block = doc.blocks.new("PAIR")
    block.add_circle((0, 0), 5)
    block.add_circle((20, 0), 5)
    doc.modelspace().add_blockref("PAIR", (0, 0))
    path = tmp_path / "pair.dxf"
    doc.saveas(path)

    w = MainWindow()
    qtbot.addWidget(w)
    w.resize(1000, 700)
    w.show()
    qtbot.waitExposed(w)
    assert w.open_dxf(path)
    click(qtbot, w, (5.0, 0.0))
    assert len(w.view.selected_contours()) == 1
    assert "share a handle" in w.selection_text.toPlainText()
