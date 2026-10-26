# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Prof. de départ
forms-start-depth-hover = Où la coupe commence, vers le bas depuis le dessus du brut
forms-end-depth = Prof. finale
forms-end-depth-hover = Où la coupe s'arrête, vers le bas depuis le dessus du brut
forms-through = Traversant
forms-through-hover = Traverser le brut : { $depth }, son épaisseur plus la profondeur permise en dessous (Paramètres d'usinage > Brut)
forms-oversize = Surcote
forms-oversize-disabled = Un contour sur la ligne n'a pas de côté où usiner plus
forms-step-down = Prof. de passe
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = prof. de passe

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
forms-vcarve-cut-one-pass = depuis { $start }, au plus { $end } de profondeur, une passe
forms-vcarve-cut-steps = depuis { $start }, au plus { $end } de profondeur, passes de { $step }

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
forms-passes = Passes
forms-in-steps = en plusieurs passes

# The operation's tool. "T3" is tool number 3.
forms-tool = Outil
forms-tool-not-in-project = T{ $tool } (pas dans le projet)
forms-from-library = Depuis votre bibliothèque :
forms-use-cutting-values = Utiliser les valeurs de coupe de l'outil
forms-use-cutting-values-hover = Depuis la bibliothèque : avance { $feed } / plongée { $plunge } { $feed_unit }, prof. de passe { $step_down }, pas latéral { $stepover }

forms-section-feeds = Avances
forms-feed = Avance
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
forms-tab-width = largeur
forms-tab-height = hauteur
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

# An imported DXF.
forms-file = Fichier
forms-units = Unités
forms-dxf-inch = pouce (converti en mm)
forms-shapes = Formes
forms-section-placement = Position, rotation et échelle
forms-update-dxf = Mettre à jour depuis un DXF…
forms-update-dxf-hover = Une nouvelle version du dessin : les opérations gardent leurs calques
# A button (verb).
forms-remove = Retirer
forms-remove-dxf-hover = Retirer ce DXF et ses formes

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
forms-job-settings = Paramètres d'usinage
forms-section-stock = Brut
forms-stock-size = Taille X × Y
forms-thickness = Épaisseur
forms-max-overcut = Prof. max sous le brut
forms-max-overcut-hover = Jusqu'où une coupe peut descendre sous le brut (dans le martyr). Les coupes dans cette limite donnent un avertissement ; plus profondes, elles sont refusées.
forms-stock-position = Position du brut
forms-stock-corner-at-origin = coin à l'origine du dessin
forms-stock-centred-on-origin = centré sur l'origine du dessin
forms-z-zero = Zéro Z
forms-z-zero-top = dessus du brut
forms-z-zero-bottom = dessous du brut
forms-machine-summary = { $post }, Z de sécurité { $safe_z }, { $spindle }
forms-section-machine = Machine
forms-from-machine = De la machine
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
forms-dxf-reset-position = Revenir à la position du DXF (rotation et échelle restent)
forms-bottom-left = En bas à gauche
forms-bottom-left-hover = Le coin du dessin à { $margin } du coin inférieur gauche du brut
# A button: centre the drawing on the stock.
forms-centre = Centrer
forms-centre-hover = Le centre du dessin sur le centre du brut
forms-dxf-rotation-hover = En degrés, sens anti-horaire depuis le DXF tel que dessiné, tout angle, autour du centre du dessin
forms-dxf-reset-rotation = Revenir à l'angle du DXF (0°)
forms-scale = Échelle
forms-scale-hover = Depuis le DXF tel que dessiné, autour du centre du dessin. Tailles d'outil, profondeurs et attaches ne changent pas d'échelle.
forms-dxf-reset-scale = Revenir à la taille du DXF (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Pièce { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Usiner
forms-part-cut-hover = Désactivé : la pièce et ses opérations restent dans le projet, sans être usinées
forms-part-cut-check = cette pièce et ses opérations
forms-active = Active
forms-active-hover = Les nouvelles opérations vont dans la pièce active. Clic droit sur une pièce pour l'activer
forms-active-yes = oui : les nouvelles opérations vont ici
forms-activate = Activer
forms-operations = Opérations
forms-operations-none = aucune pour l'instant
forms-copies-one = une
forms-copies = Copies
forms-parts-note = Les pièces sont usinées l'une après l'autre, chacune avec ses copies.
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

# Font styles, headings in the font list.
forms-font-plain = Simple
forms-font-script = Manuscrite
forms-font-display = Fantaisie
forms-explode-dxf = Décomposer…
forms-explode-dxf-hover = Transformer les formes de ce DXF en formes dessinées modifiables ; le lien avec le fichier DXF disparaît
