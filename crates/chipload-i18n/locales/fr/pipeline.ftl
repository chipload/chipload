# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses, planning notes, the time estimate, chip loads and the
# drawing's facts. A length or feed argument brings its own unit ("12.5 mm",
# "1200 mm/min").

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = le post-processeur { $post } n'écrit que du G-code en mm (sa configuration ne liste pas "{ $units }" sous units)

## Planning notes

pipeline-drawing-outside-stock = Le dessin dépasse du brut
# $count is the number of copies. $x0 and $y0 are numbers without their unit; $x1, $y1
# and $stock_y bring it; $stock_x is a number without it.
pipeline-copies-beyond-stock = Les { $count } copies usinent de X{ $x0 } à { $x1 }, de Y{ $y0 } à { $y1 }, au-delà du brut ({ $stock_x } × { $stock_y }) ; vérifiez le brut ou les copies de la pièce.
# As pipeline-copies-beyond-stock, for one part of several; $part is its name.
pipeline-part-copies-beyond-stock = Pièce { $part } : les { $count } copies usinent de X{ $x0 } à { $x1 }, de Y{ $y0 } à { $y1 }, au-delà du brut ({ $stock_x } × { $stock_y }) ; vérifiez le brut ou les copies de la pièce.
# $label is an operation's label, $layer a layer's name, $sources where the layer's shapes
# come from (DXF and picture names, joined with pipeline-and).
pipeline-layer-clash = { $label } : le calque "{ $layer }" est dans { $sources } ; l'opération les usine tous. Limitez-la à un seul (sélectionnez le calque sous ce DXF dans le Navigateur)
# Names joined: "a.dxf and b.dxf".
pipeline-and = { $list } et { $last }
# Where shapes come from: a DXF's or picture's name, or one without a name.
pipeline-source-named = { $name }
pipeline-source-dxf = DXF { $id }
pipeline-source-image = image { $id }
pipeline-source-drawn = formes dessinées

## The time estimate (after planning and generating)

# A duration: "45 s", "12 min 30 s", "1 h 05 min". $minutes after hours has two digits.
pipeline-duration-seconds = { $seconds } s
pipeline-duration-minutes = { $minutes } min { $seconds } s
pipeline-duration-hours = { $hours } h { $minutes } min
# The durations are pipeline-duration-… texts; $changes is the number of tool changes.
pipeline-estimate = Temps estimé : { $total } (coupe { $cutting }, rapides { $rapids }, broche { $spindle }){ $changes ->
    [0] {""}
    [one] , plus { $changes } changement d'outil
   *[other] , plus { $changes } changements d'outil
}
# The lines under it start with spaces, written {"  "}: keep them, they indent the line.
# $name is the part's name.
pipeline-estimate-part = {"  "}Pièce { $number } ({ $name }) : { $time }
# $label is an operation's label.
pipeline-estimate-operation = {"  "}{ $label } : { $time }
# An operation under its part: indented deeper.
pipeline-estimate-part-operation = {"    "}{ $label } : { $time }
# One G-code file of a project written one file per tool.
pipeline-estimate-file = {"  "}Fichier { $number } : { $time }
# $rate is a feed ("5000 mm/min").
pipeline-estimate-rapids-assumed = {"  "}Aux avances programmées (rapides à { $rate }, supposé : réglez l'avance max de la machine pour une meilleure estimation) ; l'accélération et les coins rallongent le temps réel.
pipeline-estimate-rapids = {"  "}Aux avances programmées (rapides à l'avance max de la machine, { $rate }) ; l'accélération et les coins rallongent le temps réel.

## Chip load

pipeline-chip-load-title = Avance par dent (à l'avance de coupe) :
# $label is an operation's label, $load a pipeline-chip-load… text.
pipeline-chip-load-line = {"  "}{ $label } : { $load }
# $load brings its unit ("0.042 mm/tooth").
pipeline-chip-load = { $load }
pipeline-chip-load-no-flutes = réglez le nombre de dents de l'outil pour voir l'avance par dent
pipeline-chip-load-no-rpm = réglez la vitesse de broche de l'outil pour voir l'avance par dent
pipeline-chip-load-no-tool = aucun outil
# The hover text next to an operation's chip load. Tool diameters $small_tool (a number)
# and $large_tool (with its unit); chip loads $wood_low, $aluminium_low and
# $aluminium_high (numbers) and $wood_high (with its unit).
pipeline-chip-load-hint = Avance par dent : l'épaisseur du copeau que prend chaque arête de coupe, avance / (vitesse de broche × dents). Trop fin, l'outil frotte et chauffe ; trop épais, il peut casser. Elle dépend de la matière, de l'outil et de la machine ; à titre indicatif, le bois et les plastiques avec des outils de { $small_tool }–{ $large_tool } prennent souvent { $wood_low }–{ $wood_high }, l'aluminium { $aluminium_low }–{ $aluminium_high }. Vérifiez les valeurs du fabricant de l'outil.

## The drawing's facts (after importing)

# $units is "mm" or "inch", as the DXF gives them or the user chose them.
pipeline-report-units = Unités : { $units } ({ $from ->
    [dxf] du dxf
    [svg] du svg
   *[user] choix de l'utilisateur
})
# Numbers without their unit, except $height, which brings it.
pipeline-report-extents = Étendue : X { $x0 } .. { $x1 }, Y { $y0 } .. { $y1 } ({ $width } x { $height })
# $items lists DXF entity types with their counts ("LINE 4, ARC 2").
pipeline-report-imported = Importé : { $items }
pipeline-report-flattened = Aplati : { $items }
pipeline-report-ignored = Ignoré : { $items }
pipeline-report-no-issues = Problèmes : aucun
pipeline-report-issues = Problèmes ({ $count }) :
# One issue of the import (a dxf-… text), indented.
pipeline-report-issue = {"  "}[{ $severity ->
    [warning] avertissement
   *[error] erreur
}] { $issue }
pipeline-report-contours = Contours ({ $count }) :
# A row of the contour table, in a fixed-width font: $id, $state and $layers are padded
# with spaces to line up. The coordinates are numbers without their unit.
pipeline-report-contour = {"  "}#{ $id } { $state } calque { $layers } X { $x0 } .. { $x1 }  Y { $y0 } .. { $y1 }
# A contour's state in the table: keep it short.
pipeline-report-contour-state = { $state ->
    [open] ouvert
    [invalid] INVALIDE
    [hole] trou
   *[outer] extérieur
}

## The G-code checker's findings (check-… texts), placed in the program

# A finding on a line of the program; $message is the finding.
pipeline-check-line = ligne { $line } : { $message }
# A finding in one of several G-code files: $file is the file's number.
pipeline-check-file = fichier { $file } : { $message }
# A finding on a line of one of several G-code files.
pipeline-check-file-line = fichier { $file }, ligne { $line } : { $message }

## Errors: a change or run the project refuses

# The independent checker refused the program; each finding follows on its own line.
pipeline-error-check-failed = La vérification du G-code a échoué :
# One more finding (a pipeline-check-… text) on a line of its own, indented, after $list.
pipeline-error-check-finding =
    { $list }
    {"  "}{ $finding }
# Words of the command line or another library, kept as they are.
pipeline-text = { $text }

## Reports of changes to the drawing

# $x0 and $y0 are lengths without their unit, $x1 and $y1 with it; $width and $length the
# stock's size, the first without its unit.
pipeline-stock-fit = Le dessin (X{ $x0 } à { $x1 }, Y{ $y0 } à { $y1 }) dépasse du brut ({ $width } × { $length }) ; vérifiez le brut dans Paramètres d'usinage.

## Placing a DXF: moving, turning and scaling it

# $count is how many DXFs the project has, $dxfs their numbers and names ("1 part-a, 2 part-b").
pipeline-place-which-dxf = le projet a { $count } DXF ({ $dxfs }) : indiquez lequel placer
pipeline-no-dxf = il n'y a pas de DXF { $id }
pipeline-place-nothing-imported = le projet n'a pas de dessin importé à déplacer
# What is placed, when the project has a single DXF (otherwise it is the DXF's name).
pipeline-place-the-drawing = le dessin importé
# $dx and $dy are numbers as typed, without a unit.
pipeline-place-cannot-move = impossible de déplacer le dessin de ({ $dx }, { $dy })
pipeline-place-already-there = Le dessin est déjà à cet endroit ; rien n'a été déplacé.
# $what is pipeline-place-the-drawing or a DXF's name; $degrees without its unit.
pipeline-place-turned = { $what } tourné de { $degrees }° { $direction ->
    [ccw] dans le sens anti-horaire
   *[cw] dans le sens horaire
}
pipeline-place-reset = { $what } remis là où le DXF le place
# $factor is a scale factor ("2", "0.5").
pipeline-place-scaled = { $what } mis à l'échelle ×{ $factor }
# $scaled is pipeline-place-scaled; $width is a length without its unit, $height with it.
pipeline-place-scaled-size = { $scaled } (maintenant { $width } × { $height })
# $dx is a length without its unit, $dy with it.
pipeline-place-moved = { $what } déplacé de X{ $dx } Y{ $dy }
# $done is one of the texts above; $placement a pipeline-placement text.
pipeline-place-done = { $done } ; il est maintenant { $placement }.
pipeline-place-moved-starts = { $count ->
    [one] { $count } point de départ déplacé avec lui.
   *[other] { $count } points de départ déplacés avec lui.
}
pipeline-place-moved-corners = { $count ->
    [one] { $count } choix de coin déplacé avec lui.
   *[other] { $count } choix de coin déplacés avec lui.
}
pipeline-place-moved-tabs = { $count ->
    [one] { $count } attache placée à la main déplacée avec lui.
   *[other] { $count } attaches placées à la main déplacées avec lui.
}
pipeline-place-drawn-stay = Les formes dessinées dans Chipload restent où elles sont.
pipeline-place-other-dxfs-stay = Les autres DXF restent où ils sont.
pipeline-place-work-zero-stays = L'origine pièce reste au point X{ $x }, Y{ $y } ; vérifiez qu'elle est toujours là où vous la voulez.
pipeline-place-margin = la marge doit être 0 ou plus, reçu { $got }
pipeline-place-stock-follows = le brut est centré sur le dessin et le suit, aligner le dessin dessus ne change donc rien : choisissez d'abord une position du brut à l'origine du dessin
pipeline-cut-outline = Contour de la coupe : { $width } × { $length } (X { $x0 } … { $x1 }, Y { $y0 } … { $y1 }, outil compris) : le plus petit brut qui contient le travail.
# $got is a number as typed, in degrees, without a unit.
pipeline-place-rotation = la rotation doit être un nombre de degrés, reçu { $got }
pipeline-place-scale = l'échelle doit être un nombre supérieur à 0, reçu { $got }
# Scale factors: the result, and the smallest and largest allowed.
pipeline-place-scale-range = cela mettrait le dessin à l'échelle ×{ $factor } par rapport au DXF ; elle doit rester entre ×{ $min } et ×{ $max }
# $what is "width" or "height"; $got a length without its unit.
pipeline-place-size = la { $what ->
    [height] hauteur
   *[width] largeur
} doit être supérieure à 0, reçu { $got }
pipeline-place-no-size = le dessin importé n'a pas de { $what ->
    [height] hauteur
   *[width] largeur
} à mettre à l'échelle
# How a DXF is placed: "scaled ×2, turned 90°, moved X12.000 Y-3.500 from the DXF".
pipeline-placement-as-dxf = comme dans le DXF
pipeline-placement-from-dxf = { $parts } par rapport au DXF
pipeline-placement-and = { $list }, { $last }
pipeline-placement-scaled = mis à l'échelle ×{ $factor }
pipeline-placement-turned = tourné de { $degrees }°
# $dx and $dy are lengths without their unit.
pipeline-placement-moved = déplacé de X{ $dx } Y{ $dy }
# After scaling: $within and $off are lengths, $factor the scale factor.
pipeline-scale-curves = { $count ->
    [one] { $count } forme issue de splines ou d'ellipses a été approchée
   *[other] { $count } formes issues de splines ou d'ellipses ont été approchées
} à { $within } près à la taille d'avant ; mises à l'échelle ×{ $factor }, elles peuvent s'écarter de leurs courbes jusqu'à { $off }. Mettez à jour depuis le DXF pour les approcher à nouveau à cette taille.
pipeline-scale-contours = { $was ->
    [one] { $was } contour fermé
   *[other] { $was } contours fermés
} avant, { $now } maintenant : les écarts sont joints à { $within } près à la nouvelle taille. Vérifiez le rapport d'import et les sélections des opérations.
# --units is an option of the command line.
pipeline-scale-units = Si les unités du DXF sont fausses, définissez-les plutôt lors de la mise à jour (--units), pour que le projet les enregistre.

## Adding, updating and removing a DXF

# $file is the DXF's file, $id its number, $name its name in the project.
pipeline-dxf-added = { $file } ajouté comme { $kind } { $id } "{ $name }" : { $count } { $count ->
    [one] forme
   *[other] formes
}.
# $operations are operation numbers ("2, 5"); $name a DXF's name.
pipeline-dxf-pinned = { $count ->
    [one] L'opération { $operations } continue d'usiner { $name } seulement : le nouveau DXF a aussi ses calques.
   *[other] Les opérations { $operations } continuent d'usiner { $name } seulement : le nouveau DXF a aussi leurs calques.
}
pipeline-dxf-removed = { $kind } { $id } "{ $name }" retiré avec { $count ->
    [one] sa forme
   *[other] ses { $count } formes
}.
# $label is an operation's label; $shapes shape numbers ("S3, S4").
pipeline-dxf-shapes-gone = { $label } : les formes { $shapes } n'existent plus ; choisissez à nouveau sa géométrie
pipeline-dxf-gone = { $label } : son DXF n'existe plus ; choisissez à nouveau sa géométrie
pipeline-dxf-updated = Mis à jour depuis { $file } : { $unchanged } inchangé(s), { $changed } modifié(s), { $added } ajouté(s), { $removed } supprimé(s).
# $placement is a pipeline-placement text.
pipeline-dxf-placed-as-before = Placé comme avant : { $placement }.
pipeline-dxf-geometry-changed = { $label } : géométrie modifiée (formes { $shapes }) ; vérifiez son parcours d'outil
pipeline-dxf-layer-updated = { $label } : le calque { $layer } utilise maintenant le dessin mis à jour ; vérifiez son parcours d'outil

## Pictures and texts

# $number is an operation's number.
pipeline-left-out-found = Opération { $number } : { $count ->
    [one] sa forme exclue retrouvée.
   *[other] ses { $count } formes exclues retrouvées.
}
pipeline-left-out-lost = Opération { $number } : { $count ->
    [one] { $count } forme exclue non retrouvée
   *[other] { $count } formes exclues non retrouvées
} : choisissez à nouveau les formes à exclure.
pipeline-no-image = il n'y a pas d'image { $id }
# $error is the PNG writer's own words.
pipeline-image-png-failed = écriture de l'image : { $error }
# drawing.images is a place in the project file; $error the decoder's own words.
pipeline-image-unreadable = drawing.images : impossible de lire l'image de l'image { $id } : { $error }
# $file is the picture's file; $width a length without its unit, $height with it;
# $threshold a grey level (0–255); $picked "yes" when the threshold was picked from the
# picture; $lines "yes" for centre lines, "no" for outlines.
pipeline-image-traced = { $file } vectorisé ({ $width } × { $height }, seuil { $threshold }{ $picked ->
    [yes] , choisi
   *[no] {""}
}) : { $count } { $lines ->
    [yes] { $count ->
        [one] ligne médiane
       *[other] lignes médianes
    }
   *[no] { $count ->
        [one] forme
       *[other] formes
    }
} sur le calque { $layer }.
pipeline-image-nothing-black = Rien n'est noir : changez le seuil, ou inversez si l'image est claire sur fond sombre.
# $narrow is a length without its unit, $wide with it.
pipeline-image-strokes = Traits de { $narrow } à { $wide } de large : usinez-les avec un suivi de tracé.
pipeline-image-closed-lines = { $count ->
    [one] { $count } ligne fermée : une opération Contour ou une poche peut l'usiner, comme tout autre contour.
   *[other] { $count } lignes fermées : une opération Contour ou une poche peut les usiner, comme tout autre contour.
}
pipeline-image-dots = { $count ->
    [one] { $count } point trop petit pour une ligne médiane exclu.
   *[other] { $count } points trop petits pour une ligne médiane exclus.
}
# $area is the smallest piece's area in mm², without its unit.
pipeline-image-specks = { $count ->
    [one] { $count } tache plus petite que { $area } mm² supprimée.
   *[other] { $count } taches plus petites que { $area } mm² supprimées.
}
pipeline-image-black-at-border = Le noir touche le bord de l'image et est fermé le long de celui-ci ; si c'est le fond, inversez.
pipeline-image-without-arcs = { $count ->
    [one] { $count } forme gardée en lignes : ses arcs passaient trop près d'une autre forme.
   *[other] { $count } formes gardées en lignes : leurs arcs passaient trop près d'une autre forme.
}
pipeline-image-unsimplified = { $count ->
    [one] { $count } forme gardée telle que vectorisée : le lissage la faisait se croiser.
   *[other] { $count } formes gardées telles que vectorisées : le lissage les faisait se croiser.
}
pipeline-no-text = il n'y a pas de texte { $id }
# $text is the text's first line in quotes; $error why (a toolpath-error text).
pipeline-text-join-failed = { $text } : impossible de fusionner les contours des lettres qui se chevauchent : { $error }
# $text is the text's first line in quotes; $height its height; $width the box's width
# without its unit, $tall its height with it.
pipeline-text-wrote = { $text } écrit en { $font }, { $height } de haut ({ $width } × { $tall }) : { $count } { $count ->
    [one] forme
   *[other] formes
} sur le calque { $layer }.
# $missing are the characters the font has no letter for.
pipeline-text-missing = { $font } n'a pas de lettre pour "{ $missing }" : exclue.
pipeline-text-dropped = { $count ->
    [one] { $count } contour trop petit pour être usiné exclu.
   *[other] { $count } contours trop petits pour être usinés exclus.
}

## Errors: operations and tools

# `tool use` is a command of the command line: keep it as written.
pipeline-no-tool-use = le projet n'a pas d'outil { $tool } ; copiez-en un depuis votre bibliothèque avec `tool use`
pipeline-no-tool = le projet n'a pas d'outil { $tool }
# $number is the operation's number as listed, $count how many the project has.
pipeline-no-operation = il n'y a pas d'opération { $number } ; le projet en a { $count }
pipeline-tab-no-ring = l'opération { $number } n'a pas de contour à moins d'un diamètre d'outil de { $at } ; `chipload op tabs` liste ses attaches
pipeline-tab-none-there = l'opération { $number } n'a pas d'attache à moins d'un diamètre d'outil de { $at } ; `chipload op tabs` liste ses attaches
pipeline-tab-no-tabs = l'opération { $number } n'a pas d'attaches : seul un contour avec attaches en a à placer
pipeline-corner-none-there = l'opération { $number } n'a pas de coin en { $at } ; `chipload op corners` liste ses coins
pipeline-corner-not-profile-or-pocket = l'opération { $number } n'a pas de coins à choisir : seuls les contours et les poches ont des dogbones
pipeline-not-a-pocket = l'opération { $number } n'est pas une poche ; seules les poches font de la reprise des restes
# $reason is one of the pipeline-rest-… texts.
pipeline-operation-refused = opération { $number } : { $reason }
pipeline-rest-dogbones = elle a des dogbones, et les dogbones et la reprise des restes ne peuvent pas encore être combinés ; les coins en dogbone n'ont pas besoin de reprise
pipeline-rest-not-endmill = l'outil { $tool } n'est pas une fraise
pipeline-rest-before-not-endmill = l'outil { $tool } qui la précède n'est pas une fraise
pipeline-rest-not-smaller = l'outil { $tool } ({ $diameter }) n'est pas plus petit que l'outil { $before } ({ $before_diameter }) qui le précède
# $dxfs lists the project's DXFs by number and name ("1 part-a, 2 part-b").
pipeline-update-which-dxf = le projet a { $count } DXF ({ $dxfs }) : indiquez lequel mettre à jour
# `tool use` is a command of the command line: keep it as written.
pipeline-surface-no-endmill = le projet n'a pas de fraise : un surfaçage en demande une (copiez-en une avec `tool use`)
# $type is the tool's type as written in the project file (drill, vbit).
pipeline-surface-not-endmill = l'outil { $tool } est de type { $type } ; un surfaçage demande une fraise

## Errors: drawn shapes

pipeline-shape-not-drawable = seuls les cercles, rectangles et polylignes peuvent être dessinés
# $kind is the shape's kind as written in the project file (circle, rectangle, polyline).
pipeline-shape-kind = la forme { $id } est de type { $kind } ; son type ne peut pas changer
pipeline-no-shapes-given = aucune forme indiquée
pipeline-shape-twice = la forme { $id } est indiquée deux fois
pipeline-no-shape = il n'y a pas de forme { $id }
pipeline-shape-imported-copy = la forme { $id } a été importée du DXF ; seules les formes dessinées peuvent être copiées ou mises en miroir
pipeline-shapes-no-extent = les formes n'ont aucune étendue
pipeline-shape-imported-transform = la forme { $id } n'est pas une forme dessinée : seules les formes dessinées peuvent être déplacées, tournées ou mises à l'échelle (décomposez d'abord son DXF)
pipeline-construction-imported = la forme { $id } n'est pas une forme dessinée : seules les formes dessinées peuvent être de la géométrie de construction (décomposez d'abord son DXF)
pipeline-construction-used = { $label } désigne de la géométrie de construction, qui n'est jamais usinée : elle ne peut pas s'exécuter tant qu'elle n'usine pas d'autres formes.
pipeline-transform-not-numbers = le déplacement, la rotation et le centre doivent être des nombres
pipeline-shapes-copy-imported = seules les formes dessinées peuvent être copiées, pas les formes importées
# "Update drawing from DXF" is the command in the app's File menu.
pipeline-shape-imported-change = la forme { $id } a été importée du DXF et ne peut pas être modifiée ici ; modifiez le dessin et utilisez Mettre à jour le dessin depuis un DXF

## Errors: parts and copies

pipeline-no-part = Aucune pièce { $id } dans le projet
pipeline-part-name-empty = Le nom d'une pièce ne peut pas être vide
pipeline-part-name-taken = Une autre pièce s'appelle « { $name } »
pipeline-last-part = Un projet a besoin d'au moins une pièce : c'est la dernière
pipeline-part-has-operations = La pièce { $name } a { $count ->
    [one] { $count } opération : déplacez-la d'abord vers une autre pièce, ou retirez-la avec elle
   *[other] { $count } opérations : déplacez-les d'abord vers une autre pièce, ou retirez-les avec elle
}
pipeline-part-no-operation = Aucune opération { $number } dans le projet
pipeline-part-cuts-nothing = La pièce n'usine rien
pipeline-copies-gap = L'écart doit être 0 ou plus, reçu { $got }

## Errors: the simulation

# $min is a length without its unit, $max and $got with it.
pipeline-simulate-cell-size = la taille de cellule doit être entre { $min } et { $max }, reçu { $got }
# $cell is the size of one cell, a length.
pipeline-simulate-too-many-cells = une cellule de { $cell } donne { $cells } cellules pour ce brut, plus de { $most } ; utilisez une cellule plus grande
pipeline-simulate-stock-too-large = le brut est trop grand pour être simulé avec des cellules de { $cell } (plus de { $most } cellules)
# $error is the PNG encoder's own words.
pipeline-simulate-png = PNG : { $error }

## Post-processors

pipeline-post-no-settings = pas de dossier des réglages, donc pas de post-processeur "{ $name }"
pipeline-post-no-folder = Pas de post-processeur "{ $name }" : il n'est pas intégré, et il n'y a pas de dossier posts
# Why a project's copy of a user post can't be compared with its file.
pipeline-post-no-posts-folder = pas de dossier posts
# What a post-processor is, one line each (Settings, This computer). $name, $extension,
# the paths and the units (mm, inch) and modes (files, program) are written as in its file.
pipeline-post-name = Nom : { $name }
pipeline-post-builtin = Intégré
pipeline-post-yours = Le vôtre
pipeline-post-settings = Réglages : { $path }
pipeline-post-template = Modèle : { $path }
pipeline-post-extension = Extension de fichier : { $extension }
pipeline-post-decimals = Décimales : { $mm } (mm), { $inch } (inch)
pipeline-post-units-list = Unités G-code : { $units }
pipeline-post-arcs = Arcs : { $arcs ->
    [arcs] G2/G3
   *[lines] en petites lignes (G1)
}
pipeline-post-tool-changes = Plusieurs outils : { $modes }
# $seconds is a time in seconds, without its unit.
pipeline-post-dwell = Temporisation de la broche : { $seconds } s
pipeline-post-no-dwell = Temporisation de la broche : aucune

## The tool library and machine profiles

# $error is why the library file couldn't be read (a job-library-error text).
pipeline-library-defaults = { $error } (valeurs intégrées par défaut utilisées)
pipeline-library-no-profile = aucun profil de machine nommé « { $name } »
# What a new project started with. $how is "profile" (asked for) or "default".
pipeline-library-machine = machine : { $name } ({ $how ->
    [default] profil par défaut
   *[profile] profil
})
pipeline-library-tool = outil { $tool } : { $name } (outil par défaut)
# $error is why the post can't be used (a post-error text).
pipeline-library-post-unusable = post-processeur « { $name } » : { $error }
pipeline-library-unknown-post = Machine « { $name } » : ce Chipload n'a pas de post-processeur « { $post } » ; les projets qui l'utilisent ne peuvent pas produire de G-code tant qu'il ne l'a pas

## Recovery copies

# $reason is the operating system's words, or the text below.
pipeline-recovery-io = Impossible d'écrire la copie de récupération { $path } : { $reason }
pipeline-recovery-locked = déjà verrouillée

## Dragging a picture's or text's box

pipeline-box-turn-centre = la poignée de rotation est sur le centre
pipeline-box-no-size = la boîte n'a pas de taille
# $min is the smallest size, a length.
pipeline-box-too-small = elle ne peut pas être plus petite que { $min }
pipeline-box-no-corner = une boîte n'a pas de coin { $corner }

## G-code files

# One of several G-code files: $name is its file name, $operations the numbers of the
# operations it cuts ("1, 3").
pipeline-gcode-file = { $name } (outil { $tool }, opérations { $operations })

## Explode a DXF, join close ends (docs/specs/explode-and-join.md)

pipeline-dxf-exploded = { $kind } { $id } « { $name } » décomposé en { $count } { $count ->
    [one] forme dessinée
   *[other] formes dessinées
}. Le projet ne connaît plus le fichier { $kind }.
# $shapes shape numbers ("S3, S4"); $tolerance a length.
pipeline-explode-split = { $count ->
    [one] La forme { $shapes } était en morceaux qui ne se rejoignent pas : chaque morceau est maintenant une forme.
   *[other] Les formes { $shapes } étaient en morceaux qui ne se rejoignent pas : chaque morceau est maintenant une forme.
}
pipeline-explode-circle-start = { $count ->
    [one] Le cercle { $shapes } commence maintenant à son point de droite, dans le sens antihoraire, comme les cercles dessinés : sa coupe peut commencer ailleurs sur lui.
   *[other] Les cercles { $shapes } commencent maintenant à leur point de droite, dans le sens antihoraire, comme les cercles dessinés : leurs coupes peuvent commencer ailleurs sur eux.
}
pipeline-explode-dropped = { $count ->
    [one] La forme { $shapes } n'avait que des segments plus courts que { $tolerance } et a disparu.
   *[other] Les formes { $shapes } n'avaient que des segments plus courts que { $tolerance } et ont disparu.
}
# $label is an operation's label.
pipeline-explode-selects = { $label } sélectionne maintenant les formes { $shapes }.
pipeline-explode-no-dxf = le projet n'a aucun DXF à décomposer
pipeline-explode-which-dxf = le projet a { $count } DXF ({ $dxfs }) : indiquez lequel décomposer
pipeline-join-gap-range = L'écart doit être de { $least } à { $most }, reçu { $got }.
pipeline-join-imported = La forme S{ $id } vient d'un DXF : décomposez d'abord le DXF.
pipeline-join-none = Aucune extrémité jointe (écart { $gap }).
pipeline-joined = { $pieces } formes jointes en { $count }, dont { $closed } fermées ; plus grand écart fermé { $widest }.
pipeline-joined-touching = { $pieces } formes jointes en { $count }, dont { $closed } fermées ; leurs extrémités se touchaient.
pipeline-join-open = { $count } { $count ->
    [one] extrémité reste ouverte
   *[other] extrémités restent ouvertes
}.
# $at a point, $nearest a length.
pipeline-join-open-end = L'extrémité en { $at } : l'autre extrémité la plus proche est à { $nearest }.
pipeline-join-open-alone = L'extrémité en { $at } : aucune autre extrémité sur son calque.
pipeline-join-branch = Trois extrémités ou plus se rejoignent en { $at } : pas de jonction à cet endroit.
pipeline-edit-imported = S{ $id } vient d'un DXF, d'une image ou d'un texte : éclatez le DXF, ou dessinez la forme, pour la modifier
pipeline-offset-distance = le décalage doit être supérieur à 0, reçu { $got }
pipeline-offset-vanishes = il ne reste rien de S{ $id } à { $distance } à l'intérieur
pipeline-offset-side = S{ $id } est { $closed ->
    [closed] fermée : décalez-la à l'extérieur ou à l'intérieur
   *[open] ouverte : décalez-la à gauche ou à droite
}
pipeline-offset-failed = S{ $id } n'a pas pu être décalée : { $why }
pipeline-new-shapes-cut = { $count ->
    [one] La nouvelle forme est
   *[other] Les { $count } nouvelles formes sont
} sur le calque { $layer }, que { $ops ->
    [one] l'opération { $operations } usine
   *[other] les opérations { $operations } usinent
}.
pipeline-corner-none = S{ $id } n'a pas de coins à arrondir ou chanfreiner
pipeline-corner-not = le coin { $corner } de S{ $id } n'est pas entre deux côtés droits
pipeline-corner-straight = S{ $id } continue tout droit en { $corner } : il n'y a pas de coin
pipeline-corner-too-large = c'est trop grand pour les côtés de S{ $id } : au plus { $most }
pipeline-corner-size = le rayon ou la distance doit être supérieur à 0
pipeline-trim-nothing-crosses = rien ne croise S{ $id } à cet endroit (une forme fermée demande deux croisements) : supprimez-la plutôt
pipeline-trim-everything = ajuster S{ $id } à cet endroit n'en laisserait rien : supprimez-la plutôt
pipeline-extend-not-straight = seule une extrémité droite de S{ $id } peut être prolongée
pipeline-extend-nothing-ahead = rien ne se trouve devant cette extrémité de S{ $id }
pipeline-extend-closed = S{ $id } est fermée : elle n'a pas d'extrémité à prolonger
pipeline-break-at-end = c'est une extrémité de S{ $id } : il n'y a rien à couper
pipeline-combine-two = il faut au moins deux formes pour les combiner
pipeline-combine-open = S{ $id } est ouverte : seules les formes fermées peuvent être combinées
pipeline-combine-empty = { $op ->
    [subtract] il ne reste rien de S{ $id } : les autres formes la couvrent entièrement
   *[other] les formes n'ont aucune partie en commun : il ne reste rien
}
pipeline-combine-failed = les formes n'ont pas pu être combinées : { $why }
pipeline-array-count = un réseau compte de 2 à { $most } formes avec l'original, reçu { $got }
pipeline-array-spacing = les copies seraient les unes sur les autres : donnez un espacement différent de 0 là où il y a plus d'une colonne ou d'une rangée
pipeline-array-angle = l'angle doit être supérieur à 0 et d'au plus 360 degrés, reçu { $got }
pipeline-align-two = il faut au moins deux formes pour aligner, ou le brut comme repère
pipeline-distribute-three = il faut au moins trois formes pour répartir
pipeline-shape-gone = S{ $id } n'existe plus : { $ops ->
    [one] l'opération { $operations } l'utilisait ; choisissez de nouveau sa géométrie
   *[other] les opérations { $operations } l'utilisaient ; choisissez de nouveau leur géométrie
}.

## DXF export (docs/specs/dxf-export.md)

pipeline-export-nothing = il n'y a rien à exporter : aucune forme n'est affichée là
pipeline-export-no-layer = le dessin n'a pas de calque { $layer }
pipeline-export-unverified = le DXF n'a pas été écrit : S{ $id } ne s'est pas relue à l'identique ({ $why })
pipeline-export-summary = { $count ->
    [one] 1 entité
   *[other] { $count } entités
} sur { $layers ->
    [one] 1 calque
   *[other] { $layers } calques
}, en { $units ->
    [inch] pouces
   *[mm] mm
}.
pipeline-export-renamed = Le calque { $from } s'appelle { $to } dans le DXF : un nom de calque DXF ne peut pas contenir certains de ses caractères.
