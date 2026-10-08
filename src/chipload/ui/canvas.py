"""2D drawing view: contours, stock outline and toolpath. Display only, no geometry logic."""

from __future__ import annotations

import math

from PySide6.QtCore import QPointF, QRectF, Qt, Signal
from PySide6.QtGui import QColor, QMouseEvent, QPainter, QPainterPath, QPen, QWheelEvent
from PySide6.QtWidgets import (
    QGraphicsEllipseItem,
    QGraphicsItem,
    QGraphicsPathItem,
    QGraphicsScene,
    QGraphicsSimpleTextItem,
    QGraphicsView,
    QWidget,
)

from chipload.core.geometry.chain import Contour
from chipload.core.toolpath.model import Program

BACKGROUND = "#1f2124"
OUTER = "#e8eaed"
HOLE = "#8ab4f8"
OPEN = "#fbbc04"
INVALID = "#f28b82"
STOCK = "#5f6368"
FEED = "#34a853"
FEED_REJECTED = "#ea4335"
RAPID = "#9aa0a6"
LABEL = "#fdd663"
AXIS_X = "#f28b82"
AXIS_Y = "#81c995"

ARROW_SPACING_MM = 40.0  # distance between direction arrows along the cutting path
AXIS_LENGTH_MM = 15.0
ZOOM_STEP = 1.25


def _pen(color: str, width: float = 1.5, style: Qt.PenStyle = Qt.PenStyle.SolidLine) -> QPen:
    pen = QPen(QColor(color), width)
    pen.setCosmetic(True)  # constant on-screen width at any zoom
    pen.setStyle(style)
    return pen


class DrawingView(QGraphicsView):
    """Y-up view in millimetres. Wheel zooms around the cursor, drag pans."""

    cursor_moved = Signal(float, float)

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setScene(QGraphicsScene(self))
        self.setRenderHint(QPainter.RenderHint.Antialiasing)
        self.setDragMode(QGraphicsView.DragMode.ScrollHandDrag)
        self.setTransformationAnchor(QGraphicsView.ViewportAnchor.AnchorUnderMouse)
        self.setBackgroundBrush(QColor(BACKGROUND))
        self.setMouseTracking(True)
        self.scale(1, -1)  # drawing coordinates are Y up
        self._layers: dict[str, list[QGraphicsItem]] = {"stock": [], "contours": [], "path": []}
        self._add_axes()

    # -- content

    def show_contours(self, contours: tuple[Contour, ...]) -> None:
        self._clear("contours")
        for contour in contours:
            points = contour.points()
            if len(points) < 2:
                continue
            path = QPainterPath(QPointF(*points[0]))
            for x, y in points[1:]:
                path.lineTo(x, y)
            if contour.closed:
                path.closeSubpath()
            self._add("contours", QGraphicsPathItem(path), _contour_pen(contour))

    def show_stock(self, bounds: tuple[tuple[float, float], tuple[float, float]] | None) -> None:
        self._clear("stock")
        if bounds is None:
            return
        (x0, y0), (x1, y1) = bounds
        path = QPainterPath()
        path.addRect(QRectF(x0, y0, x1 - x0, y1 - y0))
        self._add("stock", QGraphicsPathItem(path), _pen(STOCK, 1.0, Qt.PenStyle.DashLine))

    def show_toolpath(self, program: Program | None, rejected: bool = False) -> None:
        """Feeds solid, rapids dashed, numbered start points, arrows in the cutting direction."""
        self._clear("path")
        if program is None:
            return
        feeds = QPainterPath()
        rapids = QPainterPath()
        x: float | None = None
        y: float | None = None
        order = 0
        since_arrow = ARROW_SPACING_MM  # so the first cut of every ring gets an arrow
        for move in program.moves:
            nx = move.x if move.x is not None else x
            ny = move.y if move.y is not None else y
            if nx is None or ny is None or (nx, ny) == (x, y):
                x, y = nx, ny
                continue
            if move.kind == "rapid":
                if x is not None and y is not None:
                    rapids.moveTo(x, y)
                    rapids.lineTo(nx, ny)
                order += 1
                self._start_marker(nx, ny, order)
                since_arrow = ARROW_SPACING_MM
            elif x is not None and y is not None:
                feeds.moveTo(x, y)
                feeds.lineTo(nx, ny)
                length = math.hypot(nx - x, ny - y)
                if since_arrow >= ARROW_SPACING_MM:
                    self._arrow((x + nx) / 2, (y + ny) / 2, nx - x, ny - y, rejected)
                    since_arrow = 0.0
                since_arrow += length
            x, y = nx, ny
        self._add("path", QGraphicsPathItem(rapids), _pen(RAPID, 1.0, Qt.PenStyle.DashLine))
        self._add("path", QGraphicsPathItem(feeds), _pen(FEED_REJECTED if rejected else FEED, 2.0))

    def clear_all(self) -> None:
        for layer in self._layers:
            self._clear(layer)

    def fit(self) -> None:
        rect = self.scene().itemsBoundingRect()
        if rect.isEmpty():
            return
        margin = max(rect.width(), rect.height()) * 0.05
        self.fitInView(
            rect.adjusted(-margin, -margin, margin, margin), Qt.AspectRatioMode.KeepAspectRatio
        )
        # Room to pan well beyond the drawing.
        span = max(rect.width(), rect.height()) * 3
        self.setSceneRect(rect.adjusted(-span, -span, span, span))

    def item_count(self, layer: str) -> int:
        return len(self._layers[layer])

    # -- interaction

    def wheelEvent(self, event: QWheelEvent) -> None:
        steps = event.angleDelta().y() / 120
        factor = ZOOM_STEP**steps
        self.scale(factor, factor)

    def mouseMoveEvent(self, event: QMouseEvent) -> None:
        point = self.mapToScene(event.position().toPoint())
        self.cursor_moved.emit(point.x(), point.y())
        super().mouseMoveEvent(event)

    # -- helpers

    def _add(self, layer: str, item: QGraphicsPathItem, pen: QPen) -> None:
        item.setPen(pen)
        self.scene().addItem(item)
        self._layers[layer].append(item)

    def _clear(self, layer: str) -> None:
        for item in self._layers[layer]:
            self.scene().removeItem(item)
        self._layers[layer].clear()

    def _add_axes(self) -> None:
        for dx, dy, color in ((AXIS_LENGTH_MM, 0.0, AXIS_X), (0.0, AXIS_LENGTH_MM, AXIS_Y)):
            path = QPainterPath(QPointF(0, 0))
            path.lineTo(dx, dy)
            item = QGraphicsPathItem(path)
            item.setPen(_pen(color, 2.0))
            self.scene().addItem(item)

    def _start_marker(self, x: float, y: float, order: int) -> None:
        dot = QGraphicsEllipseItem(-4, -4, 8, 8)
        dot.setFlag(QGraphicsItem.GraphicsItemFlag.ItemIgnoresTransformations)
        dot.setPos(x, y)
        dot.setPen(_pen(LABEL, 1.5))
        self.scene().addItem(dot)
        # Child of the dot, so its position is in screen pixels: up and to the right.
        label = QGraphicsSimpleTextItem(str(order), dot)
        label.setBrush(QColor(LABEL))
        label.setPos(6, -18)
        self._layers["path"].append(dot)

    def _arrow(self, x: float, y: float, dx: float, dy: float, rejected: bool) -> None:
        chevron = QPainterPath(QPointF(-7, -5))
        chevron.lineTo(0, 0)
        chevron.lineTo(-7, 5)
        item = QGraphicsPathItem(chevron)
        item.setFlag(QGraphicsItem.GraphicsItemFlag.ItemIgnoresTransformations)
        item.setPos(x, y)
        # Screen space is Y down, so a scene angle of theta (CCW, Y up) is -theta on screen.
        item.setRotation(-math.degrees(math.atan2(dy, dx)))
        item.setPen(_pen(FEED_REJECTED if rejected else FEED, 2.0))
        self.scene().addItem(item)
        self._layers["path"].append(item)


def _contour_pen(contour: Contour) -> QPen:
    if not contour.closed:
        return _pen(OPEN, 1.5, Qt.PenStyle.DashLine)
    if contour.invalid_reason is not None:
        return _pen(INVALID, 2.5)
    if (contour.nesting_depth or 0) % 2 == 1:
        return _pen(HOLE)
    return _pen(OUTER)
