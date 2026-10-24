# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = DXF ohne Einheiten; wie gewählt in { $units } gelesen
# $max is the largest height found.
dxf-nonzero-z = { $count ->
    [one] { $count } Element hat
   *[other] { $count } Elemente haben
} ein Z ungleich null (max. |Z| { $max }); Z wurde verworfen
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source }: { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Blöcke tiefer verschachtelt als { $most }
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }): { $reason }
dxf-skip-coordinate = eine Koordinate ist keine Zahl oder größer als { $most }
dxf-skip-fit-points = hat nur Anpassungspunkte; exportieren Sie es erneut mit Kontrollpunkten
dxf-skip-not-flat = nicht parallel zur XY-Ebene
# $block is the block's name, in quotes.
dxf-skip-no-block = Block { $block } existiert nicht
# $radius is in the drawing's units, without one.
dxf-skip-radius = Radius { $radius } ist nicht positiv
dxf-spline-degree = Grad muss mindestens 1 sein
dxf-spline-few-points = { $count } Kontrollpunkte sind zu wenige für Grad { $degree }
dxf-spline-knots = { $knots } Knoten, erwartet { $expected } für { $count } Kontrollpunkte vom Grad { $degree }
dxf-spline-knots-order = Knoten sind nicht aufsteigend
dxf-spline-weights = Anzahl der Gewichte weicht von der Anzahl der Kontrollpunkte ab

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = { $type } der Länge null auf Layer { $layer } entfernt ({ $handle })
dxf-duplicate = Doppeltes { $type } auf Layer { $layer } entfernt ({ $handle })
dxf-branch-point = Mehr als zwei Segmente treffen sich in einem Punkt
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = Kontur { $contour } auf { $layers } ist offen: endet bei { $start } und { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Kontur { $contour }: { $reason }
dxf-ring-few-points = weniger als 3 verschiedene Punkte
dxf-ring-self-intersecting = schneidet sich selbst
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = faltet sich auf sich selbst zurück: ihre Kanten kommen sich bis auf { $within } nahe, bei ({ $x }, { $y })
dxf-ring-no-area = umschließt keine Fläche

## Why a DXF can't be imported

# $path is the file; $reason the operating system's or the DXF reader's own words, or one
# of the texts below.
dxf-error-unreadable = DXF { $path } kann nicht gelesen werden: { $reason }
dxf-error-parser-crashed = der DXF-Parser ist bei dieser Datei abgestürzt
# $scale is a factor without a unit.
dxf-error-scale = Import im Maßstab { $scale } nicht möglich
# $INSUNITS is the name of the DXF header variable: keep it as written.
dxf-error-units-missing = DXF hat keine Einheiten ($INSUNITS=0); geben Sie mm oder Zoll ausdrücklich an
# $code is the number the DXF gives its units.
dxf-error-units-unsupported = DXF $INSUNITS={ $code } wird nicht unterstützt (nur mm, Zoll oder ohne Einheit). Exportieren Sie die Zeichnung erneut in mm oder Zoll.
# $dxf and $expected are "mm" or "inch".
dxf-error-units-mismatch = DXF gibt { $dxf } an, erwartet war aber { $expected }. Korrigieren Sie die Zeichnung oder wählen Sie die Einheiten ausdrücklich; Einheiten werden nie geraten.
