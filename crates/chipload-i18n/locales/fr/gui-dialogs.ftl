# File dialogs, typed-path prompts, the About window and the window title
# (crates/chipload-gui/src/app/dialogs.rs).

# Help > User guide, until the guide is published online. Keep the file names as they are.
dialogs-user-guide = Le guide de l'utilisateur (en anglais) est livré avec chaque version sous le nom chipload-guide-<version>.zip (ouvrir index.html). Depuis les sources : packaging/docs/build-book.sh.

# Names of file types in the system's file dialogs.
dialogs-filter-projects = Projets Chipload
dialogs-filter-dxf = Dessins (DXF, SVG)

dialogs-selector-copied = Sélecteur copié dans le presse-papiers

dialogs-about-title = À propos de Chipload
dialogs-about-appimage = L'AppImage Linux contient aussi libxkbcommon et libxcb-xkb (licences de type MIT) ; leurs fichiers de copyright sont dans usr/share/doc/chipload/bundled à l'intérieur de l'AppImage.
dialogs-third-party = Licences tierces

# Asked when a DXF doesn't say whether it was drawn in millimetres or inches.
dialogs-units-title = Unités du dessin
dialogs-units-unknown = { $file } n'indique pas ses unités.
dialogs-units-question = Dans quelles unités a-t-il été dessiné ?
# A button: the drawing was drawn in inches.
dialogs-inch = pouce
dialogs-units-cancelled = Dessin non importé : ses unités sont inconnues.
dialogs-px-title = Pixels par pouce
dialogs-px-unknown = { $file } ne donne sa taille qu'en pixels.
dialogs-px-question = Combien de pixels font un pouce ?
dialogs-px-96 = 96 (Inkscape, navigateurs)
dialogs-px-72 = 72 (anciens Illustrator)
dialogs-px-other = Autre :
dialogs-px-use = Utiliser
dialogs-px-cancelled = Dessin non importé : sa taille est inconnue.
dialogs-export-dxf-title = Exporter le dessin en DXF
dialogs-export-dxf-inch = En pouces (sinon mm)
dialogs-export-dxf-hidden = Les calques et groupes masqués aussi
dialogs-export-dxf-construction = La géométrie de construction aussi (sur ses propres calques)
dialogs-export-dxf-selection = Seulement les formes sélectionnées
dialogs-export-dxf-save = Exporter…
dialogs-export-dxf-path = Exporter vers un chemin…
dialogs-path-export-dxf = Le DXF à écrire :
dialogs-filter-dxf-file = Dessin DXF

dialogs-unsaved-title = Modifications non enregistrées
dialogs-unsaved-body = Le projet a des modifications non enregistrées. Les enregistrer d'abord ?
dialogs-save-hover = Enregistrer, puis continuer
dialogs-dont-save = Ne pas enregistrer
dialogs-dont-save-hover = Abandonner les modifications et continuer

# Another program saved the open project's file.
dialogs-changed-title = { $file } a changé sur le disque
dialogs-changed-body = Un autre programme a enregistré le projet. Charger sa version ? Vos modifications non enregistrées restent à un Ctrl+Z.
dialogs-load-it = Le charger
dialogs-load-it-hover = Afficher la version du fichier ; Ctrl+Z rétablit la vôtre
dialogs-keep-mine = Garder la mienne
dialogs-keep-mine-hover = Garder vos modifications ; le prochain enregistrement écrase le fichier

# Windows where a file's path is typed in: a title and what to type.
dialogs-path-open-title = Ouvrir un chemin
dialogs-path-open = Un projet .chipload, ou un .dxf ou .svg :
dialogs-path-save-title = Enregistrer le projet
dialogs-path-save = Enregistrer le projet vers :
dialogs-path-export-title = Exporter le G-code
dialogs-path-export = Exporter le G-code vérifié vers :
dialogs-path-update-title = Mettre à jour le dessin
dialogs-path-update = Le DXF ou SVG :
dialogs-path-image-title = Importer une image
dialogs-path-image = Une image (PNG, JPEG ou BMP) :
dialogs-path-add-dxf-title = Ajouter un DXF ou SVG
dialogs-path-add-dxf = Le DXF ou SVG à ajouter au projet :
dialogs-path-update-dxf-title = Mettre à jour le fichier de dessin
dialogs-path-update-dxf = La nouvelle version du DXF ou SVG :

# In the window's title bar, for a project never saved.
dialogs-title-new-project = nouveau projet

## Explode a DXF, join close ends (docs/specs/explode-and-join.md §5)

# $name is the DXF's name in the Browser.
dialogs-explode-title = Décomposer { $name } en formes
dialogs-explode-body = Ses formes deviennent des formes dessinées que vous pouvez modifier, déplacer et supprimer. Les opérations continuent d'usiner les mêmes formes. Le projet ne connaît plus le fichier DXF : il ne sera plus possible de le mettre à jour depuis le DXF.
dialogs-explode-image-body = Ses formes deviennent des formes dessinées que vous pouvez modifier, déplacer et supprimer. Les opérations continuent d'usiner les mêmes formes. L'image elle-même quitte le projet : elle ne pourra plus être vectorisée.
dialogs-explode-text-body = Ses lettres deviennent des formes dessinées que vous pouvez modifier, déplacer et supprimer. Les opérations continuent d'usiner les mêmes formes. Les mots ne pourront plus être modifiés.
dialogs-explode-join = Joindre les extrémités à moins de
dialogs-explode = Décomposer
dialogs-join-title = Joindre les extrémités
dialogs-join-body = Parmi { $count ->
    [one] la forme sélectionnée
   *[other] les { $count } formes sélectionnées
}, les polylignes ouvertes dont les extrémités sont plus proches que l'écart n'en forment plus qu'une, sur le même calque. Une chaîne qui revient à son départ est fermée.
dialogs-join-body-all = Les polylignes dessinées ouvertes dont les extrémités sont plus proches que l'écart n'en forment plus qu'une, sur le même calque. Une chaîne qui revient à son départ est fermée.
dialogs-join-gap = Joindre les extrémités à moins de
dialogs-join = Joindre

# V-carve inlays (docs/specs/vcarve-inlay.md §4)
inlay-ribbon = Marqueterie…
inlay-ribbon-hover = Une marqueterie gravée en V des formes choisies : une poche ici, et un bouchon en miroir dans une planche à part, collé dedans
inlay-menu = Nouvelle marqueterie…
inlay-title = Marqueterie gravée en V
inlay-body = Les formes choisies sont gravées en poche dans cette planche, et en bouchon, en miroir, dans un nouveau montage : une planche à part. Le bouchon est collé face en bas dans la poche, puis son dos est raboté.
inlay-vbit = Fraise en V
inlay-none = Aucune
inlay-depth = Profondeur de la poche
inlay-depth-hover = La profondeur de la poche dans la planche de base
inlay-plug-start = Profondeur de départ du bouchon
inlay-plug-start-hover = La profondeur dans sa planche où le contour du bouchon est pris : sa face se trouve à cette profondeur dans la poche, et le reste de la poche est laissé à la colle
inlay-gap = Jeu de colle
inlay-gap-hover = De combien la planche du bouchon est gravée plus profond, pour que son dos dépasse d’autant la base et que la colle chassée ait de la place
inlay-floor = Fraise droite du fond
inlay-floor-hover = Une fraise droite pour les passes de fond des deux. Le canal du bouchon est surtout du fond
inlay-thickness = Épaisseur de la planche du bouchon
inlay-thickness-hover = La planche dans laquelle le bouchon est usiné. Ce qui reste sous son canal le tient jusqu’au rabotage
inlay-make = Créer la marqueterie
inlay-no-vbit = Le projet n’a pas de fraise en V : ajoutez-en une à ses outils d’abord
inlay-made = Marqueterie créée : l’opération { $pocket } est la poche, l’opération { $plug } le bouchon, dans le montage { $setup }, sa pièce en miroir de gauche à droite.
inlay-refused = Impossible de créer la marqueterie.
sign-title = Nouvelle enseigne
sign-body = Les mots, un cadre autour qui les suit, et chaque opération qui taille l'enseigne, dans une pièce à part. Tout reste modifiable ensuite.
sign-words = Mots
sign-words-hint = Ce que dit l'enseigne ; Entrée commence une nouvelle ligne
sign-around = Autour de la sélection
sign-around-body = { $count ->
    [one] L'enseigne entoure la forme sélectionnée, sculptée avec la fraise en V. Le cadre la suit, y compris une image retracée.
   *[other] L'enseigne entoure les { $count } formes sélectionnées, sculptées avec la fraise en V. Le cadre les suit, y compris une image retracée.
}
sign-around-words = Écrire des mots à la place
sign-around-words-hover = Faire l'enseigne autour de nouveaux mots, comme sans sélection
sign-font = Police
sign-height = Hauteur
sign-style = Style
sign-style-hover = Gravée : les lettres sculptées en V. En relief : le fond à l'intérieur de la bordure la plus intérieure abaissé autour des lettres. Marqueterie : les lettres incrustées d'une seconde planche.
sign-style-engraved = Gravée
sign-style-raised = En relief
sign-style-inlay = Marqueterie
sign-relief = Profondeur du fond
sign-relief-hover = La profondeur à laquelle le fond autour des lettres en relief est taillé, avec la fraise droite
sign-edge = Bord
sign-edge-chamfer = Chanfrein
sign-edge-hover = Un chanfrein de cette largeur autour du contour, taillé avec la fraise en V avant la découpe de l'enseigne
sign-vbit = Fraise en V
sign-end-mill = Fraise droite
sign-library-tool-hover = De votre bibliothèque d'outils : copié dans le projet quand vous créez l'enseigne
sign-board = Planche
sign-fit-board = Ajuster autour de l'enseigne
sign-fit-board-hover = Le brut ajusté autour du dessin avec cette marge, de cette épaisseur. Désactivé : le brut reste tel quel.
sign-thickness = Épaisseur
sign-board-margin = Marge
sign-adds = Ajoute : { $operations }
sign-make = Créer l'enseigne
sign-no-tools = Choisissez une fraise en V et une fraise droite (ajoutez-les d'abord aux outils du projet).
sign-made = Enseigne créée : { $count } opérations dans une pièce à part. Changez les mots et le cadre suit.
sign-refused = Impossible de créer l'enseigne.
inlay-mark-gap = dos, { $gap } au-dessus
inlay-mark-surface = surface
inlay-mark-face = face du bouchon, { $depth }
inlay-mark-pocket = poche, { $depth }
