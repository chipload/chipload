# Messages of the DXF import (crates/chipload-dxf): issues found in a drawing, listed in
# the drawing's facts under the report. Names from the DXF (entity types such as LINE or
# INSERT, layers, handles, block names) are never translated. A length argument brings
# its own unit ("0.010 mm").

## Reading the DXF

# $units is "mm" or "inch".
dxf-units-chosen = Le DXF n'a pas d'unités ; { $units } utilisé, comme choisi
# $max is the largest height found.
dxf-nonzero-z = { $count ->
    [one] { $count } entité a un Z non nul (|Z| max { $max }) ; le Z a été supprimé
   *[other] { $count } entités ont un Z non nul (|Z| max { $max }) ; le Z a été supprimé
}
# An issue found when the DXF was read, as the project file keeps it: its words, in
# English.
dxf-stored-issue = { $text }
# An issue of one of several DXFs, after the DXF's name.
dxf-source-issue = { $source } : { $issue }
# $most is the deepest blocks may be nested.
dxf-block-depth = Imbrication de blocs plus profonde que { $most }
# An entity that is skipped: its type, its handle, and why ($reason, one of the
# dxf-skip-… and dxf-spline-… texts).
dxf-entity-skipped = { $kind } ({ $handle }) : { $reason }
dxf-skip-coordinate = une coordonnée n'est pas un nombre ou dépasse { $most }
dxf-skip-fit-points = n'a que des points d'ajustement ; réexportez l'entité avec des points de contrôle
dxf-skip-not-flat = pas parallèle au plan XY
# $block is the block's name, in quotes.
dxf-skip-no-block = le bloc { $block } n'existe pas
# $radius is in the drawing's units, without one.
dxf-skip-radius = le rayon { $radius } n'est pas positif
dxf-spline-degree = le degré doit être au moins 1
dxf-spline-few-points = { $count } points de contrôle, c'est trop peu pour le degré { $degree }
dxf-spline-knots = { $knots } nœuds, { $expected } attendus pour { $count } points de contrôle de degré { $degree }
dxf-spline-knots-order = les nœuds ne sont pas croissants
dxf-spline-weights = le nombre de poids diffère du nombre de points de contrôle

## Joining the drawing into contours

# $type is the DXF entity type, $handle its handle.
dxf-zero-length = { $type } de longueur nulle supprimé sur le calque { $layer } ({ $handle })
dxf-duplicate = { $type } en double supprimé sur le calque { $layer } ({ $handle })
dxf-branch-point = Plus de deux segments se rejoignent en un point
# $layers is the contour's layer names; $start and $end its ends ("(10.000, 5.000) mm").
dxf-open-contour = Le contour { $contour } sur { $layers } est ouvert : extrémités en { $start } et { $end }
# A closed contour that can't be machined; $reason is one of the dxf-ring-… texts.
dxf-invalid-contour = Contour { $contour } : { $reason }
dxf-ring-few-points = moins de 3 points distincts
dxf-ring-self-intersecting = auto-sécant
# $x and $y are a point's coordinates, without their unit.
dxf-ring-folds = se replie sur lui-même : ses bords sont à moins de { $within } l'un de l'autre près de ({ $x }, { $y })
dxf-ring-no-area = n'enclôt aucune surface
