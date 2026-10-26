# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Fiche de réglage : { $project }
# The line under the title, its parts joined with " · ".
sheet-post = post-processeur { $post }
sheet-written-by = écrit par { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = longueurs en { $length }, avances en { $feed }
sheet-gcode-inch = G-code en pouces (G20)

## Stock and zero
sheet-stock-and-zero = Brut et origine
sheet-stock = Brut
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, épaisseur { $thickness }
sheet-copies = Copies
sheet-part = Pièce
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Image { $id }
sheet-image-placed = { $file } sur le calque { $layer }, largeur { $width }, en X{ $x } Y{ $y }
sheet-text = Texte { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } en police { $font } sur le calque { $layer }, hauteur { $height }, en X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = le centre du brut
sheet-zero-bottom-left = le coin inférieur gauche du brut
sheet-zero-bottom-right = le coin inférieur droit du brut
sheet-zero-top-left = le coin supérieur gauche du brut
sheet-zero-top-right = le coin supérieur droit du brut
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } du coin inférieur gauche du brut
sheet-zero-right-of = { $distance } à droite
sheet-zero-left-of = { $distance } à gauche
sheet-zero-above = { $distance } au-dessus
sheet-zero-below = { $distance } au-dessous
sheet-zero-and = { $first } et { $second }
sheet-zero-outside = { $zero } (hors du brut)
# Where Z0 is.
sheet-z-top = le dessus du brut
sheet-z-bottom = le dessous du brut (le martyr)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Monter l'outil T{ $tool } ({ $what }). Régler X0 Y0 sur { $xy } et Z0 sur { $z }.
sheet-tool-change-m6 = Changement d'outil (M6) vers T{ $tool } ({ $what }), selon la façon dont votre contrôleur le gère.
sheet-change-tool = Changer pour l'outil T{ $tool } ({ $what }). Refaire Z0 avec cet outil sur { $z } ; garder X0 Y0.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Outil T{ $tool } : &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm { $spindle }.
sheet-tool = Outil T{ $tool } : { $what }, { $rpm } rpm { $spindle }.
sheet-spindle-cw = sens horaire (M3)
sheet-spindle-ccw = sens anti-horaire (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = fraise { $diameter }
sheet-drill = foret { $diameter }
# $angle in degrees.
sheet-vbit-angle = fraise à graver en V { $angle }°, { $diameter }
sheet-vbit = fraise à graver en V, { $diameter }
sheet-tip = pointe { $diameter }
sheet-flutes = { $count ->
    [one] { $count } dent
   *[other] { $count } dents
}

## The table of operations
sheet-column-operation = Opération
sheet-column-depth = Profondeur
sheet-column-feeds = Avance / plongée
sheet-column-entry = Entrée
sheet-column-tabs = Attaches
sheet-column-time = Temps
# A part's row over its operations, numbered from 1.
sheet-part-row = Pièce { $number } : { $name }
sheet-part-row-copies = Pièce { $number } : { $name }, { $count ->
    [one] { $count } copie
   *[other] { $count } copies
}
# An operation without a name.
sheet-operation = Opération { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } en descente
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Contour
sheet-side-outside = extérieur
sheet-side-inside = intérieur
sheet-side-on = sur la ligne
sheet-kind-pocket = Poche, en tours
sheet-climb = en avalant
sheet-conventional = en opposition
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = pas latéral { $percent } %
sheet-kind-vcarve = Gravure en V
sheet-kind-path = Suivi de tracé, sur la ligne
sheet-kind-holes-drilled = Perçages, percés
sheet-kind-holes-bored = Perçages, alésage hélicoïdal
sheet-kind-surface = Surfaçage, tout le brut, en allers-retours (en avalant et en opposition), pas latéral { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface } ; dépasse de { $distance } les bords du brut : écarter les brides
sheet-cuts-past = usine { $distance } au-delà de la ligne
sheet-leaves = laisse { $distance }
sheet-rest-after = reprise après T{ $tool }
sheet-rest-after-size = reprise après T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = dogbones aux coins jusqu'à { $degrees }°
sheet-tbones = T-bones aux coins jusqu'à { $degrees }°
sheet-corner-choices = { $count ->
    [one] { $count } coin choisi à part
   *[other] { $count } coins choisis un par un
}
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = { $start } à { $end }
sheet-depths-drill = { $start } à { $end } (pointe du foret)
sheet-pecks = débourrages { $peck }
sheet-one-go = en une fois
sheet-per-turn = { $step } par tour
sheet-per-pass = { $step } par passe
sheet-one-pass = une passe
sheet-finish = finition { $stock }
sheet-skin = peau { $thickness }
# The entry column.
sheet-entry-beside = descente à côté du brut
sheet-entry-straight = descente verticale
sheet-entry-plunge = plongée
sheet-entry-ramp = rampe { $degrees }°
sheet-entry-helix = hélice { $degrees }°
sheet-entry-helix-radius = hélice r { $radius }, { $degrees }°
sheet-lead = approche r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count } × { $width } de large, { $height } de haut
sheet-flanks = flancs { $degrees }°
sheet-tabs-none = aucune
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Usine jusqu'à Z{ $z } : { $depth } sous le dessus du brut
sheet-into-spoilboard = { $depth } dans le martyr
sheet-through = traverse le brut
sheet-sticks-out = { $cut }. Vérifier que l'outil dépasse suffisamment.

## Watch out: the notes and the checker's findings
sheet-watch-out = Attention
sheet-nothing-reported = Rien n'a été signalé.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Post-processeur { $post } : { $note }
sheet-check = Vérification : { $finding }

## The picture's legend
sheet-legend-file = Fichier { $number } (T{ $tool })
sheet-legend-tabs = attaches
sheet-legend-grey = Gris : les pièces. Mouvements de coupe seulement, sans déplacements rapides.

## The time
sheet-time = Temps
