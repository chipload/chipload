"""GUI entry point: `chipload-gui [drawing.dxf] [job.json]`."""

from __future__ import annotations

import sys
from pathlib import Path

from PySide6.QtWidgets import QApplication

from chipload.ui.main_window import MainWindow


def main(argv: list[str] | None = None) -> int:
    args = sys.argv if argv is None else ["chipload-gui", *argv]
    app = QApplication.instance() or QApplication(args)
    app.setApplicationName("Chipload")
    window = MainWindow()
    for arg in args[1:]:
        path = Path(arg)
        if path.suffix.lower() == ".json":
            window.open_job(path)
    for arg in args[1:]:
        path = Path(arg)
        if path.suffix.lower() == ".dxf":
            window.open_dxf(path)
    window.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
