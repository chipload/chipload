# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = DXF:en har inga enheter; använder { $units } enligt ditt val
# $max is the largest height found.
dxf-nonzero-z = { $count ->
    [one] { $count } entitet har
   *[other] { $count } entiteter har
} Z skilt från noll (max |Z| { $max }); Z togs bort
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source }: { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Block nästlade djupare än { $most }
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }): { $reason }
dxf-skip-coordinate = en koordinat är inte ett tal eller ligger bortom { $most }
dxf-skip-fit-points = har bara anpassningspunkter (fit points); exportera om den med kontrollpunkter
dxf-skip-not-flat = inte parallell med XY-planet
# $block is the block's name, in quotes.
dxf-skip-no-block = blocket { $block } finns inte
# $radius is in the drawing's units, without one.
dxf-skip-radius = radien { $radius } är inte positiv
dxf-spline-degree = graden måste vara minst 1
dxf-spline-few-points = { $count } kontrollpunkter är för få för grad { $degree }
dxf-spline-knots = { $knots } knutar, { $expected } väntades för { $count } kontrollpunkter av grad { $degree }
dxf-spline-knots-order = knutarna är inte stigande
dxf-spline-weights = antalet vikter skiljer sig från antalet kontrollpunkter

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = Tog bort { $type } med längden noll på lager { $layer } ({ $handle })
dxf-duplicate = Tog bort dubblett av { $type } på lager { $layer } ({ $handle })
dxf-branch-point = Fler än två segment möts i en punkt
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = Kontur { $contour } på lager { $layers } är öppen: slutar i { $start } och { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Kontur { $contour }: { $reason }
dxf-ring-few-points = färre än 3 skilda punkter
dxf-ring-self-intersecting = korsar sig själv
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = viker tillbaka över sig själv: dess kanter kommer inom { $within } från varandra nära ({ $x }, { $y })
dxf-ring-no-area = omsluter ingen yta

## Why a DXF can't be imported

# $path is the file; $reason the operating system's or the DXF reader's own words, or one
# of the texts below.
dxf-error-unreadable = Cannot read DXF { $path }: { $reason }
dxf-error-parser-crashed = the DXF parser crashed on this file
# $scale is a factor without a unit.
dxf-error-scale = can't import at scale { $scale }
# $INSUNITS is the name of the DXF header variable: keep it as written.
dxf-error-units-missing = DXF has no units ($INSUNITS=0); specify mm or inch explicitly
# $code is the number the DXF gives its units.
dxf-error-units-unsupported = DXF $INSUNITS={ $code } is not supported (only mm, inch or unitless). Re-export the drawing in mm or inch.
# $dxf and $expected are "mm" or "inch".
dxf-error-units-mismatch = DXF declares { $dxf } but { $expected } was expected. Fix the drawing or choose the units explicitly; units are never guessed.
