# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Prof. de départ
forms-start-depth-hover = Où la coupe commence, vers le bas depuis le dessus du brut
forms-end-depth = Prof. finale
forms-end-depth-hover = Où la coupe s'arrête, vers le bas depuis le dessus du brut
forms-groove-width = Largeur de rainure
forms-groove-width-hover = Avec une fraise en V : la largeur de la rainure en surface. La changer règle la profondeur finale.
forms-through = Traversant
forms-through-hover = Traverser le brut : la profondeur finale est son épaisseur plus la profondeur en dessous, et suit l'épaisseur quand elle change
forms-through-below = Sous le brut
forms-through-below-hover = Jusqu'où la coupe descend sous le dessous du brut, dans le martyr. Le brut du montage fixe le maximum ({ $depth })
forms-oversize = Surcote
forms-oversize-disabled = Un contour sur la ligne n'a pas de côté où usiner plus
forms-step-down = Prof. de passe
# Indented under "Passes": the depth of each pass.

# Cutting direction (milling).
forms-direction = Sens
forms-climb = en avalant
forms-conventional = en opposition

# Tabs' sloped sides.
forms-flank-angle = angle de flanc
forms-flank-angle-hover = La pente des flancs de l'attache depuis son dessus plat. 90° est une attache droite : l'outil monte et descend à la verticale. En dessous, l'outil suit la pente et ne plonge donc jamais après une attache. Descendre un flanc est une rampe : avec un outil qui ne coupe pas au centre, pas plus raide que l'angle de rampe.
forms-flank-straight = droit
forms-flank-feet = pieds de { $length } de chaque côté

# How the tool goes down into the material.
forms-entry = Entrée
forms-entry-hover = Comment l'outil descend dans la matière. Une rampe descend le long du parcours (un contour descend en spirale sur son tour, un suivi de tracé descend en allers-retours, une poche descend en rampe aller-retour), une hélice (poches) sur un petit cercle, l'outil n'a donc pas besoin de couper au centre ; les deux utilisent l'avance de coupe.
forms-entry-plunge = plongée droite
forms-entry-ramp = rampe
forms-entry-helix = hélice
forms-entry-ramp-summary = rampe { $angle }
forms-entry-helix-summary = hélice r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, approche { $radius }
forms-helix-radius = rayon d'hélice
forms-helix-radius-hover = Du cercle que suit le centre de l'outil : inférieur au rayon de l'outil, pour qu'aucun noyau ne reste debout
forms-ramp-angle-sub = angle de rampe
forms-ramp-angle = Angle de rampe
forms-ramp-angle-hover = L'outil descend en spirale le long du contour, jamais plus raide que cet angle et jamais plus d'une profondeur de passe par tour : aucune plongée dans la pièce. Chaque passe descend à cet angle, puis continue à plat ; un angle plus raide raccourcit la rampe.

# The operation kinds, in the form's title.
forms-kind-profile = Contour
forms-kind-pocket = Poche
forms-kind-vcarve = Gravure en V
forms-kind-path = Suivi de tracé
forms-kind-holes = Perçages
forms-kind-surface = Surfaçage
forms-kind-relief = Relief
forms-operation-number = Opération { $number }

forms-name = Nom
forms-geometry = Géométrie
forms-whole-stock = Tout le brut
forms-whole-stock-hover = Un surfaçage dresse tout le brut, une fois, quelles que soient les copies
forms-use-selection = Prendre la sélection
forms-use-selection-count = Prendre la sélection ({ $count })
forms-use-selection-hover = Remplacer par les contours sélectionnés sur le canevas.
forms-use-selection-disabled = Sélectionnez d'abord des contours sur le canevas

# A profile's side of the line.
forms-side = Côté
forms-side-inside = intérieur
forms-side-outside = extérieur
forms-side-on = sur la ligne
forms-side-choose = — choisir —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Coupe
forms-profile-cut-summary = { $start }–{ $end } { $unit }, passes de { $step } { $unit }, { $direction }, rampe en spirale { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, passes de { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, passes de { $step } { $unit }, sur la ligne
forms-surface-cut-summary = { $start }–{ $end } { $unit }, passes de { $step } { $unit }, pas latéral { $stepover } × D
forms-relief-cut-summary = { $start }–{ $end } { $unit }, lignes à { $angle }°, pas latéral { $stepover } × D
forms-relief-angle = Lignes à
forms-relief-zigzag = Aller-retour
forms-vcarve-cut-one-pass = depuis { $start }, au plus { $end } de profondeur, une passe
forms-vcarve-cut-passes = depuis { $start }, au plus { $end } de profondeur, { $count } passes

# A pocket's clearing, in rings.
forms-section-clearing = Évidement
forms-clearing-summary = tours, pas latéral { $stepover } × D
forms-clearing-summary-rest = tours, pas latéral { $stepover } × D, reprise après T{ $tool }
forms-stepover = Pas latéral
forms-stepover-hover = De combien chaque tour avance, en fraction du diamètre de l'outil
forms-surface-stepover-hover = De combien chaque passe avance latéralement, en fraction du diamètre de l'outil
forms-surface-end-depth-hover = Combien est enlevé sur le dessus, vers le bas depuis le dessus du brut
forms-section-entry-lead = Entrée et approche
forms-section-entry = Entrée

# V-carving.
forms-vcarve-start-depth-hover = La surface gravée, vers le bas depuis le dessus du brut (0 : le dessus ; plus pour graver dans un fond de poche)
forms-vcarve-end-depth-hover = La profondeur maximale de la pointe, depuis le dessus du brut (end_depth dans le fichier). Les traits plus larges sont gravés à cette profondeur et signalés
forms-vcarve-passes-hover = Combien de passes jusqu'à la profondeur finale. Chacune enlève autant que les autres, elles deviennent donc moins profondes en descendant
forms-vcarve-pass-depths = à { $depths }
forms-passes = Passes
forms-depth-passes = Passes en profondeur
forms-depth-passes-hover = Comment la profondeur est répartie en passes : charge égale (chaque passe enlève autant que les autres, elles deviennent donc moins profondes en descendant) ou une profondeur de passe ordinaire (pas égaux, comme pour une poche ; chaque passe enlève plus que la précédente)
forms-depth-passes-equal-load = Charge égale
forms-vcarve-step-down-hover = La profondeur maximale de chaque passe. Les passes sont égales, la dernière à la profondeur finale
forms-ignore-kinks = Ignorer les cassures
forms-ignore-kinks-hover = Les coins où le contour tourne de moins que cet angle n'ont pas de branche de la ligne médiane : la fraise en V ne remonte pas dans chaque cassure d'une courbe. À 20°, une cassure garde au plus quelques centièmes de millimètre ; 0° grave chaque coin
forms-section-plug = Bouchon
forms-plug-summary = contour à { $start }, jeu de colle { $gap }
forms-plug-start = Profondeur de départ du bouchon
forms-plug-start-hover = La profondeur du contour du bouchon dans sa planche : sa face se trouve à cette profondeur dans la poche. Plus que la profondeur de départ, moins que la profondeur finale
forms-plug-gap = Jeu de colle
forms-plug-gap-hover = De combien la planche est gravée plus bas que son contour : la profondeur finale moins celle de départ du bouchon. Le dos du bouchon dépasse d’autant la base

# The operation's tool. "T3" is tool number 3.
forms-tool = Outil
forms-tool-not-in-project = T{ $tool } (pas dans le projet)
forms-from-library = Depuis votre bibliothèque :
forms-use-cutting-values = Utiliser les valeurs de coupe de l'outil
forms-use-cutting-values-hover = Depuis la bibliothèque : avance { $feed } / plongée { $plunge } { $feed_unit }, prof. de passe { $step_down }, pas latéral { $stepover }

forms-section-feeds = Avances
forms-feed = Avance
forms-chip-load = Avance par dent
forms-chip-load-no-flutes = dents non réglées
forms-chip-load-no-rpm = vitesse de broche non réglée
forms-chip-load-missing-hover = Réglez-les dans les propriétés de l'outil : Outils dans le navigateur, puis l'outil.
forms-flutes = Dents
forms-flutes-hover = Les arêtes de coupe de l'outil, pour l'avance par dent. Décoché : inconnu.
forms-plunge-feed = Avance plongée

# Rest machining: clear only what a larger tool left.
forms-rest-after = Reprise après
forms-rest-after-hover = N'enlever que ce qu'un outil plus gros laisse dans ces formes : les coins qu'il laisse arrondis et les rainures où il ne passe pas. Usinez d'abord la poche avec cet outil
forms-rest-none = aucune
forms-rest-endmill = fraise { $diameter }

# Square inside corners.
forms-corners = Coins
forms-corners-hover = Les coins intérieurs qu'un outil rond laisse arrondis : un dogbone entre dans le coin par sa bissectrice, un T-bone le long du plus long côté (l'encoche est dans le plus court), pour qu'une pièce carrée s'emboîte jusqu'au fond
forms-corners-round = arrondis
forms-corners-dogbones = dogbones
forms-corners-tbones = T-bones
forms-corners-disabled = Un contour sur la ligne n'a pas de coins intérieurs
forms-corners-up-to = jusqu'à
forms-corners-up-to-hover = Seulement les coins aussi aigus ou plus (les coins de 90° dessinés légèrement faux comptent quand même)
forms-corners-chosen = { $count ->
    [one] { $count } coin choisi à part
   *[other] { $count } coins choisis un par un
}
forms-corners-chosen-hover = Coins cliqués sur le canevas et choisis à part, quoi que dise la ligne au-dessus
forms-corners-reset = Réinitialiser
forms-corners-reset-hover = Chaque coin reprend ce que dit la ligne au-dessus

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = percés, débourrages de { $peck }
forms-holes-drilled-once = percés en une fois
forms-holes-bore = alésage hélicoïdal, { $step } par tour, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Où s'arrête la pointe du foret, vers le bas depuis le dessus du brut : le plein diamètre s'arrête environ 0.3 × le diamètre du foret plus haut
forms-pecks = Débourrages
forms-pecks-hover = Percer à cette profondeur à la fois, en sortant du trou entre deux pour dégager les copeaux
forms-in-pecks = avec débourrages
forms-peck = débourrage
forms-step-per-turn = Pas par tour
forms-step-per-turn-hover = La descente maximale de l'hélice en un tour
forms-bore-ramp-angle-hover = L'hélice n'est jamais plus raide que cet angle
forms-drill-feeds-summary = { $feed } { $unit } en descente
forms-drill-feed = Avance perçage

# How a profile holds its part.
forms-holding-none = aucun
forms-holding-tabs = attaches
forms-holding-skin = peau d'oignon
forms-holding-skin-tabs = peau d'oignon et attaches
forms-holding-summary-tabs = { $holding } : { $count } × { $width } { $unit }, { $height } { $unit } de haut
forms-holding-summary-skin = { $summary }, peau { $thickness }
forms-section-holding = Maintien
forms-hold-the-part = Maintenir la pièce
forms-hold-the-part-hover =
    Comment une pièce découpée reste en place.
    Attaches : des ponts laissés au fond de la coupe ; la pièce reste attachée jusqu'à ce que vous la libériez.
    Peau d'oignon : une fine peau laissée tout autour pendant l'ébauche, puis un dernier tour après chaque tour de l'opération la traverse. La pièce est libre pendant ce tour : ajoutez des attaches si les pièces ne doivent pas bouger à ce moment-là.
forms-tab-count = nombre d'attaches
# Where a profile's tabs go (docs/specs/tabs-along-grain.md): X and Y name the edges.
forms-tab-placement = disposition
forms-tab-placement-hover = Où chaque contour reçoit ses attaches, sauf si vous les avez placées à la main
forms-tab-around = tout autour
forms-tab-around-hover = Réparties régulièrement autour du contour, écartées des coins
forms-tab-along-x = le long de X
forms-tab-along-x-hover = Sur les bords inférieur et supérieur du contour, qui suivent X, moitié sur chacun. Avec le fil du bois le long de ces bords, les attaches se détachent facilement ; en travers, elles tiennent mieux
forms-tab-along-y = le long de Y
forms-tab-along-y-hover = Sur les bords gauche et droit du contour, qui suivent Y, moitié sur chacun. Avec le fil du bois le long de ces bords, les attaches se détachent facilement ; en travers, elles tiennent mieux
forms-tab-width = largeur
forms-tab-height = hauteur
forms-tabs-placed = { $count ->
    [one] { $count } attache placée à la main
   *[other] { $count } attaches placées à la main
}
forms-tabs-placed-hover = Attaches glissées, ajoutées ou retirées sur le canevas ; les contours sans en reçoivent le nombre ci-dessus, réparties régulièrement
forms-tabs-reset = Réinitialiser
forms-tabs-reset-hover = Les attaches de chaque contour sont replacées selon la disposition
forms-skin-thickness = épaisseur de peau
forms-skin-thickness-hover = Laissée au-dessus de la profondeur finale jusqu'au dernier tour : au plus une profondeur de passe, et plus fine que les attaches

# A profile's finishing pass.
forms-finish-none = aucune
forms-finish-summary = { $stock } laissé, puis un tour
forms-finishing-pass = Passe de finition
forms-finishing-pass-hover = Ébaucher en laissant cette épaisseur sur la paroi, puis l'enlever en un tour léger à pleine profondeur : une paroi plus propre sans marques de passes. Le tour descend dans la rainure de l'ébauche, jamais sur la paroi finie.
forms-finishing-lap = tour de finition
forms-finishing-disabled = Un contour sur la ligne n'a pas de paroi à finir
# The material left on the wall for the finishing lap.
forms-finish-stock = surépaisseur

# A pocket's lead-in and lead-out.
forms-lead = Approche
forms-lead-hover = Un arc qui amène l'outil sur la paroi depuis la poche évidée et l'en éloigne, pour que l'outil ne s'arrête jamais contre la paroi. Une approche sans assez de place est réduite, ou omise (le rapport le signale).
forms-lead-in-out = entrée et sortie
forms-lead-radius = rayon

# The G-code's X0 Y0.
forms-work-zero = Origine pièce X Y
forms-work-zero-hover = Le point qui devient X0 Y0 dans le G-code. La géométrie ne bouge pas.
forms-zero-drawing-origin = origine du dessin
forms-zero-stock-centre = centre du brut
forms-zero-stock-bottom-left = brut en bas à gauche
forms-zero-stock-bottom-right = brut en bas à droite
forms-zero-stock-top-left = brut en haut à gauche
forms-zero-stock-top-right = brut en haut à droite
forms-zero-point = point
forms-zero-on-stock = Sur le brut
forms-zero-on-stock-hover = Cliquez un coin ou le centre : l'origine y reste quand la taille du brut change
forms-zero-drawing-origin-button = Origine du dessin
forms-zero-drawing-origin-hover = X0 Y0 là où le dessin a son propre 0,0
forms-zero-at = Position
forms-zero-at-hover = Où se trouve l'origine, en coordonnées du dessin. Saisir une valeur en fait un point à part.
forms-zero-note = Le X0 Y0 Z0 du G-code. Le déplacer ne déplace aucune coupe, seulement les nombres du programme. Dans la vue, sélectionnez-le et faites-le glisser : il s'accroche aux coins et au centre du brut et à l'origine du dessin.

# An imported DXF.
forms-file = Fichier
forms-units = Unités
forms-dxf-inch = pouce (converti en mm)
forms-shapes = Formes
forms-section-placement = Position, rotation et échelle
forms-update-dxf = Mettre à jour depuis un DXF ou SVG…
forms-update-dxf-hover = Une nouvelle version du dessin : les opérations gardent leurs calques
# A button (verb).
forms-remove = Retirer
forms-remove-dxf-hover = Retirer ce fichier de dessin et ses formes

# A picture traced into shapes.
forms-picture = Image
forms-picture-pixels = { $width } × { $height } pixels
forms-traced-as = Vectorisée en
forms-traced-outlines = contours
forms-traced-centre-lines = lignes médianes
forms-position = Position
forms-picture-reset-position = Le coin de la boîte en X0 Y0
forms-rotation = Rotation
forms-picture-rotation-hover = En degrés, sens anti-horaire, tout angle, autour du centre de l'image. L'image est vectorisée telle quelle, puis ses formes sont tournées.
forms-turn-ccw-hover = 90° sens anti-horaire
forms-turn-cw-hover = 90° sens horaire
forms-picture-reset-rotation = Comme l'image est (0°)
forms-width = Largeur
forms-picture-width-hover = La largeur de l'image avant rotation ; la hauteur suit
forms-picture-height = hauteur { $height }
forms-layer = Calque
forms-picture-layer-hover = Le calque de ses formes, pour les opérations qui le sélectionnent
forms-trace-settings = Réglages de vectorisation…
forms-trace-settings-hover = Seuil, lissage, contours ou lignes médianes, avec un aperçu
forms-remove-picture-hover = Retirer l'image et ses formes

# A text written in a built-in font.
forms-text = Texte
forms-text-hover = Ce qu'il dit ; Enter commence une nouvelle ligne
forms-font = Police
forms-text-font-missing = La police « { $font } » n'est pas sur cet ordinateur. Le texte garde ses lettres et est usiné tel quel ; pour changer ses mots ou sa taille, choisissez une autre police. Le déplacer et le tourner restent possibles.
forms-fonts-looking = Recherche des polices…
forms-font-recent = Utilisées récemment
forms-font-builtin = Intégrées
forms-font-computer = Sur cet ordinateur
forms-font-computer-none = Aucune autre police trouvée. Installez-en une, ou placez-la dans le dossier fonts de votre dossier des réglages.
forms-font-search = Rechercher
forms-font-hover = Une police intégrée : simple, manuscrite ou fantaisie (guide de l'utilisateur : Text)
forms-height = Hauteur
forms-text-height-hover = La hauteur d'une lettre majuscule
forms-anchor = Ancrage
forms-anchor-hover = Ce que désigne Position : le coin inférieur gauche de la boîte du texte, ou son centre. Un texte centré reste centré quand ses mots, sa police ou sa taille changent
forms-anchor-corner = coin inférieur gauche
forms-anchor-centre = centre
forms-text-reset-position = En X0 Y0
forms-text-rotation-hover = En degrés, sens anti-horaire, tout angle, autour du centre du texte
forms-text-reset-rotation = Horizontal (0°)
forms-align = Alignement
forms-align-hover = Comment les lignes s'alignent les unes par rapport aux autres
forms-align-left = gauche
forms-align-centre = centre
forms-align-right = droite
forms-letter-spacing = Interlettrage
forms-letter-spacing-hover = Espace en plus entre les lettres ; 0 : celui de la police
forms-line-spacing = Interligne
forms-line-spacing-hover = D'une ligne de base à la suivante, en hauteurs de majuscule
# After the number: "1.2 × height".
forms-line-spacing-suffix = × hauteur
forms-text-layer-hover = Le calque de ses lettres, pour les opérations qui le sélectionnent
forms-size = Taille
forms-remove-text-hover = Retirer le texte et ses lettres

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Machine
forms-section-stock = Brut
forms-stock-size = Taille
forms-thickness = Épaisseur
forms-material = Matériau
forms-material-hover = Vos propres avances par outil, de Cet ordinateur > Matériaux. Le choisir donne à chaque opération, et à celles ajoutées ensuite, ce matériau et ses valeurs. Le brut reste tel quel
forms-material-none = Aucun
forms-material-not-in-library = { $name } (pas dans votre bibliothèque)
forms-material-from = De { $name }
forms-material-from-hover = Les avances, la profondeur de passe et le pas latéral sont les valeurs du matériau pour cet outil
forms-material-changed = Modifié par rapport à { $name }
forms-material-use = Valeurs de { $name }
forms-material-use-hover = Usiner avec les valeurs du matériau pour cet outil
forms-material-save = Enregistrer dans { $name }
forms-material-save-hover = Garder ces avances, profondeur de passe et pas latéral comme valeurs du matériau pour cet outil, pour les prochains travaux
forms-material-not-in = { $name } n'a pas de valeurs pour cet outil
forms-material-no-values = { $name } (pas de valeurs pour cet outil)
forms-op-material-hover = Vos propres avances par outil, de Cet ordinateur > Matériaux : l'opération prend les valeurs de ce matériau pour son outil, et les suit quand vous changez d'outil
forms-max-overcut = Prof. max sous le brut
forms-max-overcut-hover = Jusqu'où une coupe peut descendre sous le brut (dans le martyr). Les coupes dans cette limite donnent un avertissement ; plus profondes, elles sont refusées.
forms-stock-position = Position du brut
forms-stock-corner-at-origin = coin à l'origine du dessin
forms-stock-centred-on-origin = centré sur l'origine du dessin
forms-stock-centred-on-drawing = centré sur le dessin
forms-stock-margin = marge
forms-stock-fits-hover = Le contour du dessin plus la marge de chaque côté, centré sur le dessin ; il suit le dessin
forms-stock-fit = Contour + marge
forms-stock-fit-hover = Centrer le brut sur le contour du dessin et le dimensionner au contour plus une marge, en le suivant
forms-stock-fixed-size = Taille fixe, centrée
forms-stock-fixed-size-hover = Une taille que vous saisissez, pour un panneau que vous voulez utiliser, centrée sur le contour du dessin
forms-drawing-size-note = Dessin { $size }
forms-zero-summary = X0 Y0 { $xy } · Z0 { $z }
forms-material-each-op = Chaque opération peut choisir le sien, sous son outil.
forms-drawing-size-hover = La taille du contour du dessin (ce qui est usiné ; sans la géométrie de construction). Le contour de la coupe, avec l'outil, figure dans le rapport après Calculer
forms-z-zero = Zéro Z
forms-z-zero-top = dessus du brut
forms-z-zero-bottom = dessous du brut
forms-machine-summary = { $post }, Z de sécurité { $safe_z }, { $spindle }
forms-section-machine = Machine
forms-from-profile = Du profil
forms-edit-here = Modifier ici
forms-edit-here-hover = Changer les valeurs de la machine pour ce projet seulement ; elle reste liée, et montre ce qui diffère
forms-safe-z-job = Z de sécurité (ce travail)
forms-safe-z-hover = La hauteur des déplacements rapides latéraux, au-dessus du brut et de ses brides

# The project's tools.
forms-tools = Outils
forms-add-from-library = Ajouter depuis la bibliothèque…
forms-new-tool = Nouvel outil
forms-tool-not-used = inutilisé
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = opérations { $operations }
forms-tool-not-used-long = Utilisé par aucune opération
forms-tool-cuts = Usine les opérations { $operations }
forms-remove-tool = Retirer l'outil
forms-remove-tool-disabled = Utilisé par les opérations { $operations } ; donnez-leur d'abord un autre outil
# An icon button that puts a value back.
forms-reset = Réinitialiser

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Revenir à la position du fichier (rotation et échelle restent)
forms-bottom-left = En bas à gauche
forms-bottom-left-hover = Le coin du dessin à { $margin } du coin inférieur gauche du brut
# A button: centre the drawing on the stock.
forms-centre = Centrer
forms-centre-hover = Le centre du dessin sur le centre du brut
forms-dxf-rotation-hover = En degrés, sens anti-horaire depuis le fichier tel que dessiné, tout angle, autour du centre du dessin
forms-dxf-reset-rotation = Revenir à l'angle du fichier (0°)
forms-scale = Échelle
forms-scale-hover = Depuis le fichier tel que dessiné, autour du centre du dessin. Tailles d'outil, profondeurs et attaches ne changent pas d'échelle.
forms-dxf-reset-scale = Revenir à la taille du fichier (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Pièce { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Usiner
forms-part-cut-hover = Désactivé : la pièce et ses opérations restent dans le projet, sans être usinées
forms-part-cut-check = cette pièce et ses opérations
forms-part-mirror = Miroir
forms-part-mirror-hover = Usiner la pièce en miroir par rapport à l'axe central de ses formes ; le dessin reste tel quel. L'intérieur reste l'intérieur, l'avalant reste en avalant
forms-part-mirror-none = Pas en miroir
forms-active = Active
forms-active-hover = Les nouvelles opérations vont dans la pièce active. Clic droit sur une pièce pour l'activer
forms-active-yes = oui : les nouvelles opérations vont ici
forms-activate = Activer
forms-operations = Opérations
forms-operations-none = aucune pour l'instant
forms-copies-one = une
forms-copies = Copies
forms-parts-note = Les pièces sont usinées l'une après l'autre, chacune avec ses copies.
forms-setup = Montage
forms-setup-cut = Usiner
forms-setup-cut-hover = Désactivé : gardé, mais aucune de ses pièces n'est usinée
forms-setup-cut-check = ce montage et ses pièces
forms-setup-parts = Pièces
forms-setup-board = Planche
forms-setup-board-hover = Le verso d'un autre montage est la même planche retournée : il partage son brut et son origine, et ses usinages sont mis en miroir
forms-setup-board-own = Sa propre planche
forms-setup-board-back = Verso de { $setup }
forms-setup-turn = Retourner
forms-setup-turn-hover = Comment la planche est retournée : autour de son axe vertical (de gauche à droite) ou horizontal (de bas en haut)
forms-setup-turn-left-right = De gauche à droite
forms-setup-turn-top-bottom = De bas en haut
forms-setup-register = Remise en place par
forms-setup-register-hover = Des goupilles dans des trous sur l'axe de retournement remettent la planche exactement en place ; un coin de butée demande une taille de planche exacte
forms-setup-register-pins = Goupilles
forms-setup-register-corner = Un coin de butée
forms-setup-dowels = Trous de goupille
forms-setup-dowels-there = Opération { $number }
forms-setup-add-dowels = Ajouter les trous de goupille
forms-setup-add-dowels-hover = Deux trous sur l'axe de retournement, dans la marge, usinés d'abord avec { $front } : à travers la planche et 8 mm dans le plateau martyr, pour des goupilles de 8 mm
forms-setup-back-side = Verso
forms-setup-back-side-hover = Le montage qui est cette planche retournée, pour usiner l'autre face
forms-setup-back-none = Aucun
forms-setup-add-back = Ajouter le verso
forms-setup-add-back-hover = Un nouveau montage : cette planche retournée, avec sa propre pièce. Dessinez le verso vu du recto, à travers la planche : ses usinages sont mis en miroir
forms-setup-dowels-open-hover = Ouvrir l'opération qui perce les trous de goupille
forms-setup-remove-dowels-hover = Retirer les trous de goupille : leur opération et leurs deux cercles
forms-setup-place-dowels = Replacer
forms-setup-place-dowels-hover = Déplacer les trous de goupille là où ils vont maintenant, à l'écart des usinages des deux faces ; un zéro sur la première goupille suit. Seulement avant d'usiner le recto : ensuite ses trous sont percés
forms-setup-dowels-none = Pas encore
forms-setup-dowels-none-hover = Le verso est remis en place par des goupilles, et { $front } n'a pas encore de trous pour elles
forms-shared-with-front = { $back } est la planche de { $front } retournée : il partage son brut et son origine. Modifiez-les là.
forms-open-front = Ouvrir { $front }
forms-thinner-after-surfacing = Sa planche fait { $thickness } d'épaisseur après le surfaçage de { $front }.
forms-setup-make-active = En faire le montage actif, pour modifier son panneau et son origine
forms-remove-setup = Retirer le montage
forms-copies-hover = Usiner la pièce plusieurs fois en grille. Les copies sont dessinées en transparence ; seul le dessin lui-même peut être sélectionné.
forms-copies-grid = plusieurs, en grille
forms-columns-rows = colonnes × rangées
# Copy to copy distance, in X and Y.
forms-pitch = pas X Y
forms-gap = écart
forms-gap-hover = Règle le pas une fois pour laisser cet écart entre les coupes des copies voisines. Le projet garde le pas.
forms-set-pitch = Régler le pas
forms-order = ordre

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Post-proc.
forms-post-yours = { $name } (le vôtre)
forms-post-folder-hover = Vos propres post-processeurs vont dans { $folder }
forms-post-builtin-hover = Les post-processeurs intégrés
forms-gcode-units = Unités G-code
forms-gcode-units-hover = Les unités dans lesquelles le G-code est écrit : mm (G21) ou pouces (G20), selon le réglage de votre contrôleur. Tout le reste du projet reste en mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = pouce (G20)
forms-spindle = Broche
forms-spindle-cw = horaire (M3)
forms-spindle-ccw = anti-horaire (M4)
forms-max-feed = Avance max
forms-tool-changes = Changements d'outil
forms-tool-changes-hover = Avec plusieurs outils : un fichier G-code par outil (changez d'outil et refaites le zéro Z entre les fichiers), ou un seul fichier avec des changements d'outil M6 que votre logiciel d'envoi gère (GRBL lui-même refuse M6)
forms-tool-change-files = un fichier par outil
forms-tool-change-program = un fichier avec M6 (votre logiciel d'envoi gère le changement)
forms-travel = Course X Y Z
forms-max-feed-not-set = non définie
forms-travel-not-checked = non vérifiée
forms-revert-hover = « { $name } » a { $value } : cliquer pour le reprendre
forms-post-yours-copied = Le vôtre, copié dans le projet
forms-post-changed = Votre fichier de post-processeur a changé
forms-post-update = Mettre à jour
forms-post-update-hover = Copier le post-processeur tel qu'il est maintenant dans le projet
forms-post-file-missing = Le vôtre ; le projet garde sa copie (le fichier n'est plus là)

# A tool's values.
forms-tool-type = Type
forms-diameter = Diamètre
forms-spindle-speed = Vitesse de broche
forms-centre-cutting = Coupe au centre
forms-yes = oui
forms-no = non
forms-unknown = inconnu
forms-angle = Angle
forms-angle-hover = L'angle inclus du V ; la gravure en V en a besoin
forms-tip = Pointe
forms-tip-hover = Le diamètre de la pointe plate ; non défini vaut pointe aiguë
forms-pointed = aiguë

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = opération { $number }
forms-rest-pass-of-named = opération { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, surcote { $clearance } { $unit }, { $direction }
forms-rest-pass = Passe de reprise
forms-rest-of = Reprise de
forms-rest-of-hover = Elle usine les formes et profondeurs de sa poche, et n'enlève que ce que l'outil précédent laisse
# A button: show the pocket.
forms-open = Ouvrir
forms-from-pocket = De la poche
forms-after-tool = Après l'outil
forms-after-tool-hover = Elle enlève ce que cet outil laisse : celui de la poche, ou de la passe précédente
forms-cut-it = Usiner
forms-cut-it-hover = Désactivé : gardée, mais pas usinée
forms-rest-pass-note = Usinée après les poches voisines de sa poche qui utilisent le même outil, toutes leurs passes de reprise ensemble : le petit outil n'est monté qu'une fois.
forms-remove-rest-pass = Retirer la passe de reprise
forms-floor-pass = Passe de fond
forms-floor-of = Fond de
forms-floor-of-hover = Elle usine les formes de sa gravure en V jusqu'à sa profondeur finale, à l'écart des flancs inclinés
forms-from-vcarve = De la gravure en V
forms-floor-pass-facts = { $start }–{ $end } { $unit }, à { $inset } { $unit } du contour
forms-floor-pass-note = Usinée avant les gravures en V voisines qui utilisent la même fraise en V, toutes leurs passes de fond ensemble. Là où la fraise droite n'atteint pas, la fraise en V reprend le long de la ligne médiane.
forms-remove-floor-pass = Retirer la passe de fond
forms-floor-hint = Laisse une partie du fond : une fraise d'au plus { $largest } le dégagerait entièrement.
forms-floor-hint-library = Dans votre bibliothèque : { $tools }

# Font styles, headings in the font list.
forms-font-plain = Simple
forms-font-script = Manuscrite
forms-font-display = Fantaisie
forms-explode-dxf = Décomposer…
forms-explode-dxf-hover = Transformer les formes de ce DXF en formes dessinées modifiables ; le lien avec le fichier DXF disparaît

## The Statistics section of an operation (docs/specs/operation-statistics.md)
forms-section-statistics = Statistiques
forms-statistics-time = Temps
forms-statistics-length-time = { $length }, { $time }
forms-statistics-cutting = Coupe
forms-statistics-cutting-hover = Mouvements en avance sur le côté : droites, arcs, rampes et hélices, mesurés en 3D
forms-statistics-plunging = Plongées
forms-statistics-plunging-hover = Mouvements en avance droit vers le bas ou le haut : plongées et perçage
forms-statistics-rapids = Rapides
forms-statistics-rapids-hover = Mouvements rapides (G0), vers le haut et le bas compris
forms-statistics-lifts = Remontées
forms-statistics-lifts-hover = Combien de fois l'outil remonte en rapide : entre les trajets, et à chaque débourrage d'un perçage par débourrage
forms-statistics-copies = Pour { $copies } copies.
forms-statistics-generate = Générez (F5) pour voir les statistiques.
forms-statistics-generating = Génération…
forms-statistics-failed = Non généré : voir le rapport.
forms-statistics-off = Désactivée : pas usinée.
forms-statistics-out-of-date = périmé
forms-statistics-not-generated = non généré
forms-statistics-off-summary = désactivée
forms-statistics-operations-hover = Les opérations et leurs passes de reprise et de fond comptées ici : celles qui sont usinées
forms-statistics-empty = Aucune opération.
forms-statistics-empty-summary = aucune opération
forms-slot-feed = Avance rainure
forms-slot-feed-hover = L'avance des anneaux sur toute la largeur : le premier anneau à chaque profondeur, et le premier de chaque branche. Vide : l'avance
forms-slot-step-down = Profondeur rainure
forms-slot-step-down-hover = Les anneaux sur toute la largeur descendent à chaque profondeur par pas au plus de cette profondeur, avant que les autres anneaux coupent toute la passe d'un coup
forms-pocket-step-down-hover = La profondeur maximale de chaque passe ; les anneaux au pas latéral la coupent d'un coup
forms-material-speed-differs = Les valeurs de { $name } sont pour { $rpm } rpm ; l'outil tourne à { $runs }

# Reliefs and their roughing pass (docs/specs/stl-relief.md §9, §14).
forms-relief-depth = Profondeur
forms-relief-depth-hover = La profondeur du relief : celle du modèle, réglée dans ses propriétés
forms-relief-stepover-hover = L'écart entre les lignes, en part du diamètre de l'outil : plus petit est plus lisse et plus lent
forms-relief-ridge = Crêtes
forms-relief-ridge-hover = La hauteur des crêtes que la fraise hémisphérique laisse entre ses lignes sur un fond plat. Sous 0,02 mm, le bois ne les montre pas ; un pas plus fin ne fait que prendre plus de temps
forms-relief-deep = { $depth } de profondeur avec ce seul outil : ajoutez une passe d'ébauche (clic droit sur le relief) pour que la fraise hémisphérique n'enlève pas tout
forms-section-rim = Bord
forms-rim-none = aucun
forms-rim-summary = { $width } autour : { $shape }, rampe à { $angle }°
forms-rim-hover = Un plat au fond du relief tout autour, puis une rampe jusqu'à la surface du panneau : le modèle ne se trouve pas dans une tranchée et chaque ligne commence à la surface
forms-rim-shape = Autour de
forms-rim-outline = le contour
forms-rim-box = un rectangle
forms-rim-width = Largeur du plat
forms-rim-width-hover = Jusqu'où le plat du fond s'étend depuis le contour ou depuis le rectangle du modèle
forms-rim-corner = Rayon des coins
forms-rim-ramp = Rampe
forms-rim-ramp-hover = La pente du plat jusqu'à la surface du panneau, depuis l'horizontale : à 45°, elle s'étend aussi loin que le relief est profond ; 90° donne une paroi droite
forms-relief-angle-hover = La direction des lignes depuis X. Des lignes en biais par rapport aux grands bords du modèle laissent moins de marques
forms-relief-zigzag-hover = Aller-retour le long des lignes, ou chaque ligne dans le même sens, en remontant entre elles
forms-roughing-pass = Passe d'ébauche
forms-roughing-of = Ébauche de
forms-roughing-of-hover = Le relief que cette passe ébauche avant la finition
forms-from-relief = Du relief
forms-roughing-facts = { $start }–{ $end } { $unit }, lignes à { $angle }°
forms-roughing-step-down-hover = L'écart entre les niveaux : la matière est enlevée un niveau à la fois
forms-roughing-allowance = Surépaisseur
forms-roughing-allowance-hover = Ce qui reste sur le modèle pour la finition, tout autour : sur les flancs raides aussi
forms-roughing-note = Usinée juste avant le relief. Elle ne coupe jamais plus large que la finition, et le programme est vérifié pour garder la surépaisseur sur le modèle.
forms-remove-roughing = Supprimer la passe d'ébauche
forms-smoothing-pass = Passe de lissage
forms-smoothing-of = Lissage de
forms-smoothing-of-hover = Le relief que cette passe lisse après son ébauche
forms-smoothing-facts = { $start }–{ $end } { $unit }
forms-smoothing-angle = Lignes à
forms-smoothing-angle-hover = La direction des lignes depuis X. En travers des lignes d'ébauche, elle enlève les marches qu'elles laissent
forms-smoothing-allowance-hover = Ce qui reste sur le modèle pour la finition, tout autour : en général celle de l'ébauche
forms-smoothing-note = Usinée après la passe d'ébauche et avant le relief, en suivant une fois la surface. Le programme est vérifié pour garder la surépaisseur sur le modèle.
forms-remove-smoothing = Supprimer la passe de lissage
forms-add-smoothing = Ajouter une passe de lissage
forms-add-smoothing-hover = L'outil d'ébauche suit encore une fois la surface, ses lignes en travers de celles-ci, et enlève les marches qu'elles laissent sur les pentes
forms-rim-hull = Plus plein
forms-rim-hull-hover = Jusqu'où le contour du bord est rempli : 0 % suit le contour du modèle avec ses creux ; davantage remplit d'abord les creux étroits, puis les plus larges ; 100 % entoure le modèle comme un élastique (son enveloppe convexe). La rampe garde son angle.
forms-picture-white-high = Le blanc est en haut
forms-picture-white-high-hover = Les parties blanches de l'image sont le dessus du relief et le noir le fond ; décochez pour une carte de profondeur dans l'autre sens
forms-picture-smoothing = Lissage
forms-picture-smoothing-hover = Un flou sur les hauteurs, son rayon sur le panneau : enlève le bruit des pixels et le grain d'une photo ; 0 garde l'image telle quelle
forms-picture-background = Fond
forms-picture-background-hover = Les gris les plus sombres jusqu'à cette part sont du fond, pas le modèle : le contour fait le tour du reste, et le bord autour. 0 % prend toute l'image
forms-picture-gamma = Gamma
forms-picture-gamma-hover = Les hauteurs courbées : au-dessus de 1 le bas plus plat et les sommets plus raides, en dessous l'inverse ; 1 comme l'image
forms-picture-kind = Type
forms-picture-kind-hover = Carte de profondeur : la luminosité de l'image est la hauteur. Gonflée : ses formes noir et blanc, chacune montant depuis son bord, pour les logos et les lettres
forms-picture-kind-depth = Carte de profondeur
forms-picture-kind-inflate = Gonflée
forms-picture-threshold = Seuil
forms-picture-threshold-hover = Les gris plus sombres que celui-ci sont noirs : les formes. Automatique le choisit d'après l'image
forms-picture-automatic = Automatique
forms-picture-invert = Inverser
forms-picture-invert-hover = Le blanc est la forme, pas le noir
forms-picture-profile = Profil
forms-picture-profile-hover = Comment une forme monte depuis son bord : arrondi, un coussin ; droit, une arête le long de chaque trait, comme ciselée
forms-picture-profile-round = Arrondi
forms-picture-profile-straight = Droit
forms-picture-reach = Portée
forms-picture-reach-hover = Jusqu'où depuis le bord une forme continue de monter : tout ce qui est plus large s'aplatit à pleine profondeur. Automatique : la demi-largeur de la forme la plus large, rien ne s'aplatit. Pour des lettres, environ un demi-trait
