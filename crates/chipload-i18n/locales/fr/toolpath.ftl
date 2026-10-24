# Messages of the planner (crates/chipload-toolpath): rules a project must meet before it
# can be cut. Most are shown after the operation's label, as "Operation 3 (Outline): …";
# see job-operation-problem. Words in the texts that name fields or values of the project
# file (safe_z, z_zero, max_feed, entry: ramp, center_cutting, …) are written in the file
# that way and are never translated. A length or feed argument brings its own unit
# ("12.5 mm", "1200 mm/min").

## The project as a whole

toolpath-no-operations = Le projet n'a aucune opération
toolpath-all-disabled = Toutes les opérations sont désactivées : rien à usiner
# $tool is a tool's number.
toolpath-tool-undefined = L'outil { $tool } n'est pas défini dans le projet
# Under the tool's spindle speed field.
toolpath-no-rpm-short = aucune vitesse de broche
toolpath-no-rpm = L'outil { $tool } n'a pas de vitesse de broche (tr/min)
# Under the machine's safe height; $top is the height of the stock's top.
toolpath-safe-z-short = doit être au-dessus du dessus du brut (Z { $top })
# $zero is "top" or "bottom", as written in the project file.
toolpath-safe-z = safe_z { $safe } doit être au-dessus du dessus du brut (Z { $top }) ; z_zero vaut '{ $zero }'

## Depths and sizes

# $below is how far below the stock's bottom it goes, $allowed the setting "max depth
# below stock".
toolpath-too-deep = la profondeur finale { $end } descend { $below } sous le brut ({ $thickness } d'épaisseur), plus que les { $allowed } permis (Prof. max sous le brut)
toolpath-clearance-radius = la surcote { $clearance } doit être inférieure au rayon de l'outil { $radius }
toolpath-finish-radius = l'épaisseur de finition { $stock } doit être inférieure au rayon de l'outil { $radius }
toolpath-tab-height = la hauteur des attaches { $height } doit être inférieure à la profondeur de coupe { $cut } (profondeur finale moins profondeur de départ)
# $field is "feed" or "plunge_feed", as written in the project file.
toolpath-max-feed = { $field } { $feed } dépasse max_feed de la machine { $max }
toolpath-helix-for-path = l'entrée en hélice est pour les poches ; un suivi de tracé peut entrer en rampe (entry: ramp)
toolpath-helix-radius = le rayon d'hélice { $radius } doit être inférieur au rayon de l'outil { $tool_radius }, sinon un noyau reste debout au milieu
# $most is a number of tool diameters, $longest that as a length.
toolpath-lead-radius = le rayon d'approche { $radius } dépasse { $most } × le diamètre de l'outil ({ $longest })

## Tools

# $tool is a tool's number, $type a kind of tool (job-tool-type).
toolpath-surface-endmill = l'outil { $tool } est de type { $type } ; un surfaçage a besoin d'une fraise
toolpath-vcarve-vbit = la gravure en V a besoin d'une fraise en V, l'outil { $tool } est de type { $type }
toolpath-vbit-no-angle = l'outil { $tool } est une fraise en V sans angle ; réglez son angle
toolpath-pocket-endmill = les poches ont besoin d'une fraise, l'outil { $tool } est de type { $type }
toolpath-path-drill = l'outil { $tool } est un foret, qui ne peut pas suivre un tracé ; utilisez une fraise ou une fraise en V
toolpath-holes-vbit = l'outil { $tool } est une fraise en V, qui ne peut pas usiner de perçages ; utilisez un foret ou une fraise
# $method is the holes' method as written in the project file, "drill" or "bore"; $type
# a kind of tool with its article (job-tool-type-article).
toolpath-holes-method = la méthode est { $method ->
    [drill] drill, qui demande un foret
   *[bore] bore, qui demande une fraise
} ; l'outil { $tool } est { $type }
# Angles in degrees.
toolpath-flank-not-centre-cutting = l'outil { $tool } ne coupe pas au centre et les flancs des attaches ({ $flank }°) sont plus raides que l'angle de rampe ({ $ramp }°) ; descendre un flanc est une rampe, rendez-les donc au plus aussi raides que { $ramp }°
toolpath-plunge-not-centre-cutting = l'outil { $tool } ne coupe pas au centre et cette opération plonge à la verticale ; entrez plutôt en rampe (entry: ramp) ou utilisez un outil qui coupe au centre

## Rest machining

# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-tool = reprise après l'outil { $tool }, que le projet n'a pas
toolpath-rest-not-endmill = reprise après l'outil { $tool }, de type { $type } ; l'outil d'une poche est une fraise
# Two tool diameters.
toolpath-rest-not-larger = la reprise après l'outil { $tool } ({ $large }) demande un outil plus gros que celui de cette poche ({ $small })
toolpath-rest-dogbones = les dogbones et la reprise des restes ne peuvent pas encore être combinés ; mettez les dogbones sur la poche du gros outil
toolpath-rest-passes-dogbones = les dogbones et les passes de reprise ne peuvent pas encore être combinés ; les coins en dogbone n'ont pas besoin de reprise
toolpath-rest-pass-no-tool = l'outil { $tool } n'est pas défini dans le projet
toolpath-rest-pass-endmill = l'outil { $tool } est de type { $type } ; une passe de reprise usine avec une fraise
toolpath-rest-pass-larger = l'outil { $tool } ({ $diameter }) doit être plus petit que l'outil { $before } ({ $before_diameter }) qui le précède
toolpath-rest-pass-no-rpm = l'outil { $tool } n'a pas de vitesse de broche (tr/min)

## Planning notes: warnings that don't stop the G-code

# A note about an operation after its label ("Operation 3 (Outline): …"); $note is one
# of the notes below.
toolpath-labelled = { $label } : { $note }
# A note about one contour of an operation: "Operation 3 (Outline), contour #4: …".
toolpath-labelled-contour = { $label }, contour #{ $contour } : { $note }
# Lists built one item at a time: "1, 2, 3" and "1, 2 and 3". $list is the items so far.
toolpath-comma = { $list }, { $item }
toolpath-and = { $list } et { $last }
# An operation's number in a list of operations.
toolpath-number = { $number }
# An operation's number and its kind in a list: "3 (pocket)".
toolpath-operation-kind = { $number } ({ $kind ->
    [profile] contournage
    [pocket] poche
    [vcarve] gravure en V
    [path] suivi de tracé
    [holes] perçages
   *[surface] surfaçage
})
# $layers is the contour's layer names, $from and $to its two ends ("(10.000, 5.000) mm").
toolpath-skipped-open = contour ouvert #{ $contour } sur { $layers } de { $from } à { $to } ignoré
toolpath-into-spoilboard = usine { $below } sous le brut (dans le martyr)
# $operations lists operation numbers with their kind: "1 (pocket), 3 (profile) and 4 (holes)".
toolpath-plunge-unknown = L'outil { $tool } n'est pas marqué center_cutting et { $count ->
    [one] l'opération { $operations } plonge
   *[other] les opérations { $operations } plongent
} à la verticale : assurez-vous qu'il coupe au centre, ou entrez en rampe (entry: ramp)
# $operations is a list of operation numbers ("1, 2 and 4"); $label the surface's label.
toolpath-surface-after = { $label } surface le brut après que { $count ->
    [one] l'opération { $operations } l'a usiné
   *[other] les opérations { $operations } l'ont usiné
} : le surfaçage vient d'habitude en premier
# $tool is the number of the larger tool the pocket rests after.
toolpath-rest-no-earlier = reprise après l'outil { $tool }, mais aucune poche précédente n'usine ces formes avec lui aussi profond ; le reste de la poche ne sera pas usiné
toolpath-rest-pass-plunge = plonge à la verticale avec l'outil { $tool }, qui n'est pas marqué comme coupant au centre ; utilisez une hélice ou une rampe s'il ne coupe pas au centre
# $groups is how many separate runs of operations use the tool; $operations their numbers
# ("1, 2, 5").
toolpath-tool-returns = L'outil { $tool } est utilisé en { $groups } groupes (opérations { $operations }) ; les placer les unes à la suite des autres évite un changement d'outil
toolpath-vbit-no-tip = l'outil { $tool } n'a pas de diamètre de pointe ; la gravure en V le considère comme pointu
# Angles in degrees.
toolpath-flank-unknown-centre-cutting = les flancs des attaches ({ $flank }°) sont plus raides que l'angle de rampe ({ $ramp }°) ; l'outil { $tool } n'est pas marqué center_cutting, assurez-vous qu'il coupe au centre

## Planning notes: profiles

# $loops is how many separate loops the contour is cut as.
toolpath-narrow-loops = l'outil ne passe pas par une partie étroite ; usiné en { $loops } boucles séparées, et de la matière reste entre elles
# $point is a start point the user set ("(10.000, 5.000) mm").
toolpath-start-far-contour = le point de départ { $point } est à { $distance } du contour #{ $contour } ; le point le plus proche est utilisé
toolpath-start-twice-contour = le point de départ { $point } est aussi sur le contour #{ $contour }, qui en a déjà un ; il est ignoré
toolpath-tabs-inside = des attaches sur un contournage intérieur retiendront la chute découpée
# $contours is a list of contour numbers ("#3, #4").
toolpath-tabs-on-curves = certaines attaches des contours { $contours } sont sur un coin ou une courbe (aucune partie droite à proximité)
# $pairs lists the tabs that meet, by number ("1 and 2, 3 and 4"); $flank is in degrees.
toolpath-tabs-joined = les attaches { $pairs } se rejoignent sous leur sommet (flancs à { $flank }°) : elles tiennent comme une seule attache plus longue
toolpath-finish-lap-cuts = le tour de finition usine jusqu'à { $most } dans les coins ou les parties étroites (épaisseur de finition { $stock })
# $style is the operation's corner style.
toolpath-corner-unreached = coin en { $point } : l'outil ne peut pas l'atteindre ; { $style ->
    [dogbone] pas de dogbone
   *[tbone] pas de T-bone
}

## Planning notes: pockets
## $contours is a list of contour numbers ("#3, #4"): the pocket's boundary and its islands.

toolpath-corners-not-rounded = contours { $contours } : coins des tours non arrondis ; les arrondir laisserait de la matière
toolpath-pocket-narrow = contours { $contours } : l'outil ne passe pas par une partie étroite ; évidé en { $areas } zones, et de la matière reste entre elles
# $width is a number without its unit; $height brings the unit; $near is a point.
toolpath-uncut = contours { $contours } : matière non usinée à { $count ->
    [one] { $count } endroit
   *[other] { $count } endroits
}, la plus grande d'environ { $width } x { $height } près de { $near } ; réduisez le pas latéral pour l'enlever
toolpath-lead-reduced = contours { $contours } : rayon d'approche réduit à { $used } (pas de place pour { $radius })
toolpath-no-lead = contours { $contours } : pas de place pour une approche (pas même { $least }) ; la paroi est usinée sans approche
toolpath-helix-reduced = contours { $contours } : rayon d'hélice réduit à { $used } (pas de place pour { $radius })
toolpath-no-helix-room = contours { $contours } : pas de place pour une hélice (pas même { $least }) ; entrée en rampe à la place
toolpath-no-helix-narrow = contours { $contours } : pas de place pour une hélice (la poche est trop étroite) ; entrée en rampe à la place
toolpath-no-helix-way = contours { $contours } : pas de place pour une hélice (son chemin vers le premier parcours passe par une paroi ou un îlot) ; entrée en rampe à la place
toolpath-start-far-wall = contours { $contours } : le point de départ { $point } est à { $distance } de la paroi ; le point le plus proche est utilisé
toolpath-start-twice-wall = contours { $contours } : le point de départ { $point } est sur une paroi qui en a déjà un ; il est ignoré
toolpath-rest-nothing-left = le plus gros outil atteint toute la poche ; il ne reste rien à reprendre
# $width is a number without its unit; $height brings the unit.
toolpath-rest-too-small = reprise des restes : la matière à { $count ->
    [one] { $count } endroit
   *[other] { $count } endroits
} est trop petite aussi pour cet outil (la plus grande d'environ { $width } × { $height })

## Planning notes: V-carving, follow paths and holes

# $across is the V-bit's diameter, $cap the deepest it carves, $wide the widest stroke it
# carves at that depth, $need the depth the widest stroke needs.
toolpath-vcarve-too-wide-bit = contours { $contours } : trop large pour la fraise en V ({ $across } de diamètre), qui grave au plus à { $cap } de profondeur ({ $wide } de large en surface) ; les bords des parties les plus larges ne sont pas atteints (il leur faut { $need }). Autorisez plus profond, utilisez un angle plus ouvert ou évidez le fond
toolpath-vcarve-too-wide = contours { $contours } : trop large pour une gravure d'au plus { $cap } de profondeur ({ $wide } de large en surface) ; les bords des parties les plus larges ne sont pas atteints (il leur faut { $need }). Autorisez plus profond, utilisez un angle plus ouvert ou évidez le fond
# $narrow is a length of centre line, $tip the V-bit's tip diameter.
toolpath-vcarve-narrow = contours { $contours } : { $narrow } de ligne médiane sont plus étroits que la pointe ({ $tip }) et ne sont pas gravés
toolpath-path-too-short = le contour #{ $contour } est trop court pour être suivi ({ $length }) ; ignoré
toolpath-groove-no-angle = l'outil { $tool } est une fraise en V sans angle : la largeur de la rainure est inconnue
toolpath-groove-past-cone = la rainure en V est plus profonde que le cône de la fraise en V : { $width } de large en haut, avec des parois droites au-dessus du cône
toolpath-groove-width = la rainure en V fait { $width } de large en haut
toolpath-holes-not-circles = { $count ->
    [one] { $count } forme qui n'est pas un cercle ignorée
   *[other] { $count } formes qui ne sont pas des cercles ignorées
}
# $diameter is the holes' diameter, $tool the tool's, $within how far they may differ.
toolpath-holes-not-drill-size = { $count ->
    [one] perçage de { $diameter } : pas au diamètre du foret de { $tool } (à { $within } près) ; non percé
   *[other] { $count } perçages de { $diameter } : pas au diamètre du foret de { $tool } (à { $within } près) ; non percés
}
toolpath-holes-too-small-to-bore = { $count ->
    [one] perçage de { $diameter } : trop petit pour être alésé par l'outil de { $tool } ; non usiné
   *[other] { $count } perçages de { $diameter } : trop petits pour être alésés par l'outil de { $tool } ; non usinés
}
toolpath-holes-core = { $count ->
    [one] { $count } perçage laisse
   *[other] { $count } perçages laissent
} un noyau libre (perçage de plus de deux fois le diamètre de l'outil) ; maintenez-le ou utilisez une poche

## Errors: why no toolpath could be made

toolpath-error-cancelled = Generation cancelled
# An error after where it happened: $context is an operation's label, or one of the
# context texts below.
toolpath-error-context = { $context }: { $error }
toolpath-error-label-contour = { $label }, contour #{ $contour }
# $part is a part's name, $copy the copy's number.
toolpath-error-part-copy = part { $part }, copy { $copy }
toolpath-error-copy = copy { $copy }
# $at is an operation's number in the cut list ("3", "3.1" for a rest pass).
toolpath-error-no-operation = No operation { $at } in the job

## Errors: selecting shapes

# $shapes are shape numbers ("S4, S7").
toolpath-error-shape-missing = shape { $shapes } is not part of any contour (deleted, or a duplicate or zero-length segment the import removed)
# $layers are layer names, separated by commas.
toolpath-error-partly-selected = contour #{ $contour } is only partly selected (layers { $layers }); fix the drawing or the selector
# $reason is why the drawing's contour can't be machined, as the DXF import found it (in English).
toolpath-error-contour-invalid = contour #{ $contour } is invalid: { $reason }
# $selector is what the operation selects, built from the toolpath-error-selector texts.
toolpath-error-selects-nothing = selector { $selector } matches nothing
# $contours are contour numbers ("#3, #4").
toolpath-error-selects-open = selector { $selector } matches only open contours ({ $contours }); this operation needs closed ones
toolpath-error-selector-layer = layer '{ $layer }'
toolpath-error-selector-shapes = shapes { $shapes }
toolpath-error-selector-and = { $first } and { $second }
# $id is the number of a DXF, a picture or a text in the project.
toolpath-error-selector-dxf = DXF { $id }
toolpath-error-selector-image = image { $id }
toolpath-error-selector-text = text { $id }
# $shapes are the shapes left out ("S3, S5").
toolpath-error-selector-but = { $source } but { $shapes }
toolpath-error-selector-of = { $what } of { $source }
# $a and $b are contour numbers.
toolpath-error-same-shape = contours #{ $a } and #{ $b } are the same shape; select only one of them

## Errors: operations and tools

toolpath-error-pass-depths = depths must satisfy 0 <= start ({ $start }) < end ({ $end }) and step_down ({ $step_down }) must be positive
# $tool is a tool's number.
toolpath-error-rest-tool = rest after tool { $tool }, which the project doesn't have
# $diameter is the hole's, $tool the tool's diameter.
toolpath-error-blind-bore = hole { $diameter } is blind and more than twice the { $tool } tool; use a pocket
# $tool is the tool's number, $type its kind as written in the project file (endmill, drill, vbit).
toolpath-error-no-hole-fits = no selected circle can be cut with tool { $tool } ({ $diameter } { $type })
toolpath-error-no-depth = no depth to cut to
toolpath-error-no-angle = tool { $tool } has no angle
# $got is an angle in degrees, without its unit.
toolpath-error-vbit-angle = the V-bit angle must be more than 0 and less than 180 degrees, got { $got }
# $got is a length, without its unit.
toolpath-error-stepover = stepover must be positive, got { $got }

## Errors: profiles, ramps and tabs

toolpath-error-ring-no-length = a profile ring has no length
toolpath-error-lap-no-course = a wall lap has no course
# $angle is in degrees, without its unit; $length is how far the ring goes around.
toolpath-error-ring-ramp = The ring is too short to ramp down { $drop } at { $angle }° ({ $length } around); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $reach is how much of the path the ramp may use.
toolpath-error-ramp-reach = The path is too short to ramp down { $drop } at { $angle }° (only { $reach } to ramp along); use a steeper ramp angle or a straight plunge with a centre-cutting tool
toolpath-error-path-ramp = The path is too short to ramp down { $drop } at { $angle }° ({ $length } long); use a steeper ramp angle or a straight plunge with a centre-cutting tool
# $angle is in degrees, without its unit.
toolpath-error-helix-turns = a helix of radius { $radius } at { $angle }° would need { $turns } turns to go down { $drop }; use a larger radius or a steeper angle
# $offset is how far inside the line the tool's centre would run.
toolpath-error-tool-does-not-fit = Tool does not fit inside the contour (its centre would run { $offset } inside the line)
toolpath-error-offset-verification = Offset failed verification (path not at tool radius from the contour); the contour may be degenerate
# $farthest is how far the finishing lap runs from the roughing path.
toolpath-error-roughing-reach = the roughing can't reach part of the wall (the finishing lap runs { $farthest } from the roughing path, a tool radius or more): use less finishing stock or a smaller tool
# $fits is how many tabs would fit.
toolpath-error-tabs-fit = { $count } tabs of { $width } don't fit on a { $length } path ({ $fits ->
    [0] not even one fits
   *[other] at most { $fits } fit
})
# $from is a distance along the ring without its unit, $to one with it.
toolpath-error-tab-carry = could not carry a tab ({ $from } to { $to } along the ring) onto the roughing ring
# $what is one of the three texts after it.
toolpath-error-tab-verification = Tab toolpath failed verification: { $what }
# $z is a height, $at a distance along the ring.
toolpath-error-tab-below = Z{ $z } at { $at } along the ring is below the tab
toolpath-error-tab-back = the tool goes back along the ring at { $at }
toolpath-error-tab-end-below = Z{ $z } at the tab end { $end } along the ring is below the tab top Z{ $top }
# $label is the operation's label; $at the corner's point.
toolpath-error-corner-leg = { $label }: the leg at corner { $at } did not go into the ring as out and back
toolpath-error-corner-cuts = { $label }: the { $style ->
    [dogbone] dogbone
   *[tbone] T-bone
} at corner { $at } would cut into contour #{ $contour }; lower the corners' max angle or change the drawing
# $x and $y are a point's coordinates, without their unit.
toolpath-error-arc-off-circle = arc output: ({ $x }, { $y }) is not on the circle it was tagged with

## Errors: pockets

# $contours are contour numbers ("#3, #4"): the pocket's boundary and its islands.
toolpath-error-islands-enclose = contours { $contours }: overlapping islands enclose an area between them, which pockets don't support yet
toolpath-error-pocket-no-fit = contours { $contours }: tool (diameter { $diameter }) does not fit in the pocket
toolpath-error-pocket-no-fit-clearance = contours { $contours }: tool (diameter { $diameter }, clearance { $clearance }) does not fit in the pocket
toolpath-error-not-nested = contours { $contours }: pocket offsets are not nested
toolpath-error-too-many-rings = contours { $contours }: more than { $most } pocket rings
toolpath-error-pocket-verification = contours { $contours }: pocket toolpath failed verification (too close to a wall or island); the drawing may be degenerate
toolpath-error-nothing-to-clear = Nothing to clear: the selected islands cover the selected boundaries (contours { $contours })
toolpath-error-island-outside = pocket offset produced an island outside every area
toolpath-error-shrink-hole = pocket offset made a hole while shrinking a single loop
toolpath-error-open-loop = pocket offset returned an open loop where a closed one was expected
toolpath-error-rest-outside = the rest machining area lies outside the pocket
toolpath-error-left-hole-outside = material left: a hole outside every piece
toolpath-error-left-holes-enclose = material left: holes enclose an area
toolpath-error-left-wrong-area = material left: a boolean gave the wrong area
toolpath-error-left-invalid = material left: invalid geometry from the offset library

## Errors: V-carving, centre lines and letters

# What a centre line is for, inside the texts below.
toolpath-error-medial-name = { $name ->
    [vcarve] V-carve
   *[centre-line] centre line
}
# $name is toolpath-error-medial-name; $error the Voronoi library's own words.
toolpath-error-medial-failed = contours { $contours }: the { $name } centre line could not be built ({ $error }); does the outline touch or cross itself?
toolpath-error-medial-cell = { $name } centre line: a cell without its chord
# $reach is how far from the origin the centre line works.
toolpath-error-medial-far = a { $name } outline at { $at } is too far from the drawing origin (the centre line works within ±{ $reach })
toolpath-error-medial-step = The { $name } centre line
toolpath-error-vcarve-verification = contours { $contours }: V-carve verification failed: the cut would cross the outline near { $at }
toolpath-error-join-gave = joining two overlapping outlines gave { $outlines } outlines
toolpath-error-join-changed = joining overlapping outlines changed what is filled near { $at }
toolpath-error-joined-cross = joined outlines cross each other
toolpath-error-outline-unreadable = an outline the geometry library can't read

## Errors: copies and parts

# $width is the cut's width; $got the pitch, without its unit.
toolpath-error-copies-dx = Copies overlap: with columns the pitch dx must be at least { $width } (the cut is { $width } wide), got { $got }
toolpath-error-copies-dy = Copies overlap: with rows the pitch dy must be at least { $height } (the cut is { $height } high), got { $got }
# $other and $copy are copies' numbers; $dx and $dy the pitch, without its unit.
toolpath-error-copies-overlap = Copies overlap: copies { $other } and { $copy } cut into each other; move them apart (pitch dx { $dx }, dy { $dy })
# $part and $other_part are parts' names; $x0 and $y0 are without their unit.
toolpath-error-parts-overlap = Part "{ $part }" copy { $copy } overlaps part "{ $other_part }" copy { $other_copy } (X{ $x0 } to { $x1 }, Y{ $y0 } to { $y1 }): move the copies apart or change the grid

## Errors: inside the geometry library

# $what is toolpath-error-step or toolpath-error-medial-step.
toolpath-error-library-failed = { $what } failed inside the geometry library
toolpath-error-step = { $step ->
    [pocket-offset] Pocket offset
    [joining-letters] Joining letters
    [checking-joined-letters] Checking joined letters
    [material-left-union] Material left union
    [material-left-offset] Material left offset
    [material-left-boolean] Material left boolean
    [comparing-shapes] Comparing shapes
    [merging-islands] Merging islands
    [subtracting-an-island] Subtracting an island
   *[offset] Offset
}
