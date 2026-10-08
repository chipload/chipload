"""Main window: open a DXF and a job, generate, inspect the result, save checked G-code.

Thin by design: everything that decides geometry or G-code is a call into core.
"""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path

from PySide6.QtCore import Qt
from PySide6.QtGui import QAction, QFontDatabase, QKeySequence
from PySide6.QtWidgets import (
    QApplication,
    QDockWidget,
    QFileDialog,
    QInputDialog,
    QLabel,
    QMainWindow,
    QPlainTextEdit,
    QWidget,
)

from chipload.cli.report import format_contours, format_import_report
from chipload.core.errors import ChiploadError, DxfUnitsMissing, GcodeCheckFailed
from chipload.core.io.dxf_import import import_dxf
from chipload.core.io.output import write_text_atomically
from chipload.core.job.loader import load_job
from chipload.core.job.models import Job
from chipload.core.pipeline import RunResult, run_job
from chipload.core.units import Units
from chipload.ui.canvas import DrawingView

UnitsPrompt = Callable[[], Units | None]


class MainWindow(QMainWindow):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("Chipload")
        self.resize(1200, 800)

        self.dxf_path: Path | None = None
        self.job_path: Path | None = None
        self.job: Job | None = None
        self.result: RunResult | None = None

        self.view = DrawingView(self)
        self.setCentralWidget(self.view)

        self.messages = QPlainTextEdit(self)
        self.messages.setReadOnly(True)
        self.messages.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
        self.messages.setFont(QFontDatabase.systemFont(QFontDatabase.SystemFont.FixedFont))
        dock = QDockWidget("Report", self)
        dock.setObjectName("report")
        dock.setWidget(self.messages)
        self.addDockWidget(Qt.DockWidgetArea.RightDockWidgetArea, dock)
        self.resizeDocks([dock], [420], Qt.Orientation.Horizontal)

        self.coords = QLabel("", self)
        self.statusBar().addPermanentWidget(self.coords)
        self.view.cursor_moved.connect(lambda x, y: self.coords.setText(f"X {x:.3f}  Y {y:.3f} mm"))

        self._build_actions()
        self._update_state()

    # -- actions

    def _build_actions(self) -> None:
        def action(text: str, shortcut: QKeySequence | str, slot: Callable[[], object]) -> QAction:
            act = QAction(text, self)
            act.setShortcut(QKeySequence(shortcut))
            act.triggered.connect(slot)
            return act

        self.open_dxf_action = action(
            "Open &DXF…", QKeySequence.StandardKey.Open, self._ask_open_dxf
        )
        self.open_job_action = action("Open &Job…", "Ctrl+J", self._ask_open_job)
        self.generate_action = action("&Generate", "F5", self.generate)
        self.save_action = action("&Save G-code…", QKeySequence.StandardKey.Save, self._ask_save)
        self.fit_action = action("&Fit View", "F", self.view.fit)
        quit_action = action("&Quit", QKeySequence.StandardKey.Quit, self.close)

        file_menu = self.menuBar().addMenu("&File")
        for act in (self.open_dxf_action, self.open_job_action, self.save_action):
            file_menu.addAction(act)
        file_menu.addSeparator()
        file_menu.addAction(quit_action)
        self.menuBar().addMenu("&Toolpath").addAction(self.generate_action)
        self.menuBar().addMenu("&View").addAction(self.fit_action)

        toolbar = self.addToolBar("Main")
        toolbar.setObjectName("main")
        for act in (
            self.open_dxf_action,
            self.open_job_action,
            self.generate_action,
            self.save_action,
            self.fit_action,
        ):
            toolbar.addAction(act)

    def _ask_open_dxf(self) -> None:
        path, _ = QFileDialog.getOpenFileName(self, "Open DXF", "", "DXF drawings (*.dxf)")
        if path:
            self.open_dxf(Path(path))

    def _ask_open_job(self) -> None:
        path, _ = QFileDialog.getOpenFileName(self, "Open Job", "", "Chipload jobs (*.json)")
        if path:
            self.open_job(Path(path))

    def _ask_save(self) -> None:
        if self.result is None or self.dxf_path is None:
            return
        suggested = self.dxf_path.with_suffix(self.result.post.file_extension)
        path, _ = QFileDialog.getSaveFileName(self, "Save G-code", str(suggested))
        if path:
            self.save_gcode(Path(path))

    def _ask_units(self) -> Units | None:
        choice, ok = QInputDialog.getItem(
            self,
            "Drawing units",
            "This DXF does not say which units it uses.\nWhat units was it drawn in?",
            [u.value for u in Units],
            0,
            False,
        )
        return Units(choice) if ok else None

    # -- operations (also called directly by tests)

    def open_dxf(self, path: Path, ask_units: UnitsPrompt | None = None) -> bool:
        """Show a drawing. Units come from the job if one is open, else from the file, else
        from the user. Never guessed."""
        self.dxf_path = path
        self._invalidate()
        return self._show_drawing(ask_units or self._ask_units)

    def open_job(self, path: Path) -> bool:
        try:
            job = load_job(path)
        except ChiploadError as exc:
            self._report([f"Cannot open job {path.name}:", str(exc)], error=True)
            return False
        self.job, self.job_path = job, path
        self._invalidate()
        self.view.show_stock(job.stock.xy_bounds())
        if self.dxf_path is not None:
            self._show_drawing(self._ask_units)
        else:
            self._report([f"Job: {path.name}", "Open a DXF to continue."])
        self.view.fit()
        return True

    def generate(self) -> bool:
        if self.job is None or self.dxf_path is None:
            self._report(["Open both a DXF and a job before generating."], error=True)
            return False
        QApplication.setOverrideCursor(Qt.CursorShape.WaitCursor)
        try:
            result = run_job(self.job, self.dxf_path)
        except GcodeCheckFailed as exc:
            self.view.show_toolpath(exc.program, rejected=True)
            self._report(["G-code check FAILED. Nothing can be saved.", *exc.errors], error=True)
            return False
        except ChiploadError as exc:
            self.view.show_toolpath(None)
            self._report(["Generation failed:", str(exc)], error=True)
            return False
        finally:
            QApplication.restoreOverrideCursor()
            self._update_state()

        self.result = result
        self.view.show_contours(result.contours)
        self.view.show_toolpath(result.program)
        lines = [
            f"Generated {result.gcode.count(chr(10))} lines for {result.post.name}. Check passed.",
            *(f"warning: {n}" for n in result.notes),
            *(f"check warning: {w.message}" for w in result.check_warnings),
            "",
            *format_import_report(result.import_report),
            *format_contours(result.contours),
        ]
        self._report(lines)
        self._update_state()
        return True

    def save_gcode(self, path: Path) -> bool:
        if self.result is None:
            return False
        try:
            write_text_atomically(path, self.result.gcode)
        except OSError as exc:
            self._report([f"Could not save {path}:", str(exc)], error=True)
            return False
        self.statusBar().showMessage(f"Saved {path}", 5000)
        return True

    # -- state

    def _show_drawing(self, ask_units: UnitsPrompt) -> bool:
        if self.dxf_path is None:
            return False
        units = Units(self.job.source.units) if self.job is not None else None
        try:
            try:
                imported = import_dxf(self.dxf_path, units)
            except DxfUnitsMissing:
                units = ask_units()
                if units is None:
                    self._report(["Drawing not opened: its units are unknown."], error=True)
                    return False
                imported = import_dxf(self.dxf_path, units)
        except ChiploadError as exc:
            self.view.show_contours(())
            self._report([f"Cannot open {self.dxf_path.name}:", str(exc)], error=True)
            return False
        self.view.show_contours(imported.contours)
        self.view.fit()
        self._report(
            [
                f"Drawing: {self.dxf_path.name}",
                *format_import_report(imported.report),
                *format_contours(imported.contours),
            ]
        )
        return True

    def _invalidate(self) -> None:
        """Inputs changed: the old toolpath and G-code no longer apply."""
        self.result = None
        self.view.show_toolpath(None)
        self._update_state()

    def _update_state(self) -> None:
        self.generate_action.setEnabled(self.job is not None and self.dxf_path is not None)
        self.save_action.setEnabled(self.result is not None)
        parts = [p.name for p in (self.dxf_path, self.job_path) if p is not None]
        self.setWindowTitle(" — ".join(["Chipload", *parts]))

    def _report(self, lines: list[str], error: bool = False) -> None:
        self.messages.setPlainText("\n".join(lines))
        if error:
            self.statusBar().showMessage(lines[0], 10000)
