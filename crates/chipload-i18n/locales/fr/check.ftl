# Findings of the independent G-code checker (crates/chipload-check): what it found wrong
# or worth a warning in a G-code program. Words of G-code (G0, G21, M6, F, P, T, X, Z…)
# are never translated. A length argument brings its own unit ("-3.000 mm"); a number
# written after a G-code letter without a unit is the program's own number.

## Reading a line

check-unbalanced-comment = Commentaire non fermé ou imbriqué
# $text is the part of the line that could not be read, in quotes.
check-cannot-parse = Impossible de lire { $text }
check-after-end = Code après la fin du programme (M30)
check-absolute-arc-centres = G90.1 (centres d'arc absolus) n'est pas pris en charge
# $word is a G or M code, such as G43 or M7.5.
check-not-supported = { $word } n'est pas pris en charge
# $letter is a letter of G-code, such as X or F.
check-word-twice = { $letter } apparaît deux fois sur une ligne
check-unsupported-word = Mot { $letter } non pris en charge
check-several-motions = Plus d'une commande de mouvement sur une ligne

## Units and modes

# $code is G20 or G21; $units the units it sets, "mm" or "inch".
check-units-changed = { $code } change les unités en { $units ->
        [inch] pouces
       *[mm] mm
    } en cours de programme
# $code is G20 or G21, $units the units it sets and $machine the machine's G-code units,
# each "mm" or "inch".
check-units-differ = { $code } ({ $units ->
        [inch] pouces
       *[mm] mm
    }) alors que les unités G-code de la machine sont { $machine ->
        [inch] pouces
       *[mm] mm
    }
# $feed is the number after F.
check-feed-not-positive = L'avance F{ $feed } doit être positive
check-relative = G91 (relatif) n'est pas pris en charge
# $tool is the number after T.
check-tool-not-whole = T{ $tool } doit être un numéro d'outil entier
check-start-without-speed = Broche démarrée après un changement d'outil sans vitesse (S)
check-dwell-without-p = Temporisation G4 sans P
check-blend-and-dwell = G64 et G4 sur une ligne : P est ambigu
# G64 P lets the controller round corners. $p is the number after P, $most a length.
check-blend-too-far = G64 P{ $p } permet au contrôleur de s'écarter du parcours de plus de { $most }
# $most is the largest number P may have.
check-blend-without-p = G64 sans P permet au contrôleur d'arrondir les angles sans limite (utilisez P { $most } au plus)
check-no-motion-mode = Mots d'axe sans mode de mouvement

## Tool changes

check-change-without-tool = Changement d'outil (M6) sans numéro d'outil (T)
# $tool is the tool number after T.
check-change-unknown-tool = Changement d'outil (M6) vers T{ $tool }, qui n'est pas un outil du projet
check-change-spindle-on = Changement d'outil (M6) avec la broche en marche (arrêtez-la d'abord avec M5)
check-change-below-safe = Changement d'outil (M6) sous la hauteur de sécurité (remontez d'abord)

## Moves

check-motion-before-units = Mouvement avant G21 (mm) ou G20 (pouces)
check-motion-before-absolute = Mouvement avant G90 (absolu)
check-rapid-below-safe = Déplacement rapide en XY alors que Z est sous la hauteur de sécurité (remontez d'abord)
# $z is a height with its unit, written after Z: "Z-3.000 mm".
check-rapid-into-material = Déplacement rapide vers Z{ $z }, sous le dessus du brut (rapide dans la matière)
check-cut-without-feed = Mouvement d'usinage sans avance définie
check-cut-spindle-off = Mouvement d'usinage avec la broche arrêtée
check-cut-unknown-position = Mouvement d'usinage depuis une position inconnue
# $z is a height with its unit, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock-too-far = Z{ $z } est sous le dessous du brut ({ $bottom }) de plus que les { $allowed } permis
check-arc-without-centre = Arc sans décalages de centre I/J
check-arc-unknown-position = Arc depuis une position inconnue
check-arc-full-circle = Arc finissant là où il commence (un cercle complet en un seul mouvement)
# $start and $end are the arc's radius at its two ends.
check-arc-radius-mismatch = Rayons d'arc différents : { $start } au début, { $end } à la fin

## The whole program

# $deepest is the lowest height, written after Z; $bottom the stock bottom's height and
# $allowed how far below it moves may go.
check-below-stock = { $count ->
    [one] { $count } mouvement descend
   *[other] { $count } mouvements descendent
} sous le dessous du brut (le plus bas Z{ $deepest }, dessous { $bottom }), dans les { $allowed } permis
check-outside-stock = { $count ->
    [one] { $count } mouvement d'usinage finit
   *[other] { $count } mouvements d'usinage finissent
} hors du brut (agrandi du rayon de l'outil)
check-no-travel = Course de la machine non définie ; les limites de course n'ont pas été vérifiées
# $axis is X, Y or Z; $span how far the program moves along it, $travel how far the
# machine can.
check-travel-exceeded = L'étendue en { $axis } ({ $span }) dépasse la course de la machine ({ $travel })
check-no-end = Pas de fin de programme (M30)
