# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = El DXF no tiene unidades; se usan { $units }, como elegiste
# $max is the largest height found.
dxf-nonzero-z = { $count ->
    [one] { $count } entidad tiene Z distinta de cero
   *[other] { $count } entidades tienen Z distinta de cero
} (|Z| máx. { $max }); se descartó la Z
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source }: { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Bloques anidados a más de { $most } niveles
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }): { $reason }
dxf-skip-coordinate = una coordenada no es un número o supera { $most }
dxf-skip-fit-points = solo tiene puntos de ajuste; vuelve a exportar con puntos de control
dxf-skip-not-flat = no es paralela al plano XY
# $block is the block's name, in quotes.
dxf-skip-no-block = el bloque { $block } no existe
# $radius is in the drawing's units, without one.
dxf-skip-radius = el radio { $radius } no es positivo
dxf-spline-degree = el grado debe ser al menos 1
dxf-spline-few-points = { $count } puntos de control son muy pocos para el grado { $degree }
dxf-spline-knots = { $knots } nodos; se esperaban { $expected } para { $count } puntos de control de grado { $degree }
dxf-spline-knots-order = los nodos no están en orden ascendente
dxf-spline-weights = el número de pesos no coincide con el de puntos de control

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = Se eliminó una entidad { $type } de longitud cero en la capa { $layer } ({ $handle })
dxf-duplicate = Se eliminó una entidad { $type } duplicada en la capa { $layer } ({ $handle })
dxf-branch-point = Más de dos segmentos se juntan en un punto
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = El contorno { $contour } en { $layers } está abierto: termina en { $start } y { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Contorno { $contour }: { $reason }
dxf-ring-few-points = menos de 3 puntos distintos
dxf-ring-self-intersecting = se corta a sí mismo
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = se pliega sobre sí mismo: sus bordes se acercan a menos de { $within } entre sí cerca de ({ $x }, { $y })
dxf-ring-no-area = no encierra ningún área

## Why a DXF can't be imported

# $path is the file; $reason the operating system's or the DXF reader's own words, or one
# of the texts below.
dxf-error-unreadable = No se puede leer { $path }: { $reason }
dxf-error-parser-crashed = el lector de DXF falló con este archivo
# $scale is a factor without a unit.
dxf-error-scale = no se puede importar a escala { $scale }
# $INSUNITS is the name of the DXF header variable: keep it as written.
dxf-error-units-missing = El DXF no tiene unidades ($INSUNITS=0); indica mm o pulgadas explícitamente
# $code is the number the DXF gives its units.
dxf-error-units-unsupported = El $INSUNITS={ $code } del DXF no es compatible (solo mm, pulgadas o sin unidades). Vuelve a exportar el dibujo en mm o pulgadas.
# $dxf and $expected are "mm" or "inch".
dxf-error-units-mismatch = El DXF declara { $dxf }, pero se esperaba { $expected }. Corrige el dibujo o elige las unidades explícitamente; las unidades nunca se adivinan.
