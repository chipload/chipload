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
