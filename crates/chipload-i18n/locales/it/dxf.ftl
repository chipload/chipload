# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = Il DXF non ha unità; uso { $units } come scelto
# $max is the largest height found.
dxf-nonzero-z = { $count } entità con Z non nulla (max |Z| { $max }); Z è stata scartata
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source }: { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Annidamento dei blocchi più profondo di { $most }
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }): { $reason }
dxf-skip-coordinate = una coordinata non è un numero o supera { $most }
dxf-skip-fit-points = ha solo punti di interpolazione (fit points); riesportala con i punti di controllo
dxf-skip-not-flat = non è parallela al piano XY
# $block is the block's name, in quotes.
dxf-skip-no-block = il blocco { $block } non esiste
# $radius is in the drawing's units, without one.
dxf-skip-radius = il raggio { $radius } non è positivo
dxf-spline-degree = il grado deve essere almeno 1
dxf-spline-few-points = { $count } punti di controllo sono troppo pochi per il grado { $degree }
dxf-spline-knots = { $knots } nodi, attesi { $expected } per { $count } punti di controllo di grado { $degree }
dxf-spline-knots-order = i nodi non sono in ordine crescente
dxf-spline-weights = il numero di pesi è diverso dal numero di punti di controllo

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = Rimossa entità { $type } di lunghezza nulla sul layer { $layer } ({ $handle })
dxf-duplicate = Rimossa entità { $type } duplicata sul layer { $layer } ({ $handle })
dxf-branch-point = Più di due segmenti si incontrano in un punto
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = Il contorno { $contour } su { $layers } è aperto: termina in { $start } e in { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Contorno { $contour }: { $reason }
dxf-ring-few-points = meno di 3 punti distinti
dxf-ring-self-intersecting = si autointerseca
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = si ripiega su se stesso: i suoi lati arrivano entro { $within } l'uno dall'altro vicino a ({ $x }, { $y })
dxf-ring-no-area = non racchiude alcuna area

## Why a DXF can't be imported

# $path is the file; $reason the operating system's or the DXF reader's own words, or one
# of the texts below.
dxf-error-unreadable = Impossibile leggere { $path }: { $reason }
dxf-error-parser-crashed = il lettore DXF è andato in crash su questo file
# $scale is a factor without a unit.
dxf-error-scale = impossibile importare con scala { $scale }
# $INSUNITS is the name of the DXF header variable: keep it as written.
dxf-error-units-missing = Il DXF non ha unità ($INSUNITS=0); indica esplicitamente mm o inch
# $code is the number the DXF gives its units.
dxf-error-units-unsupported = $INSUNITS={ $code } del DXF non è supportato (solo mm, inch o senza unità). Riesporta il disegno in mm o inch.
# $dxf and $expected are "mm" or "inch".
dxf-error-units-mismatch = Il DXF dichiara { $dxf } ma era atteso { $expected }. Correggi il disegno o scegli le unità esplicitamente; le unità non vengono mai indovinate.
