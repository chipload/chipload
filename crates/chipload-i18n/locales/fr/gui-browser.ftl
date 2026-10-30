# The Browser: the project as one tree of folders and rows (crates/chipload-gui/src/app/browser.rs).

# The panel's caption, shown in capitals.
browser-caption = Navigateur
browser-no-project = Aucun projet ouvert.
browser-new-project = Nouveau projet

# A row's eye (shown on the canvas or not) and tick (cut or not).
browser-eye-hide-hover = Le masquer sur le canevas ; il reste usiné
browser-eye-show-hover = Masqué sur le canevas (toujours usiné) : l'afficher
browser-tick-on-hover = Usiné : décocher pour l'exclure du programme
browser-tick-off-hover = Non usiné : cocher pour l'usiner

# A project tool's link to the user's tool library, at its row's right end ($name: the
# library tool's name).
browser-link-changed = { $name } (modifié)
browser-link-missing = (pas dans la bibliothèque)

# The kinds of operation, and what a new one of each does.
browser-profile = Contour
browser-profile-hover = Une opération de contour pour les contours sélectionnés
browser-pocket = Poche
browser-pocket-hover = Une opération de poche pour les contours sélectionnés
browser-vcarve = Gravure en V
browser-vcarve-hover = Une fraise à graver en V le long de la ligne médiane des contours fermés sélectionnés, à la profondeur qui rend la coupe aussi large que la forme
browser-follow-path = Suivi de tracé
browser-follow-path-hover = L'outil sur la ligne des formes sélectionnées, ouvertes ou fermées : gravure, rainures et traits
browser-holes = Perçages
browser-holes-hover = Les cercles sélectionnés : percés avec un foret, ou alésés en hélice avec une fraise
browser-surface = Surfaçage
browser-surface-hover = Tout le brut dressé à plat avec une large fraise : surfacer le martyr ou dresser le brut. Aucune sélection requise

# Job settings and Tools.
browser-job-settings = Paramètres d'usinage
browser-job-settings-hover = Brut, machine, post-processeur et outils
browser-work-zero = Origine pièce
browser-work-zero-hover = Où se trouve le X0 Y0 Z0 du G-code. Cliquez pour le régler, ou sélectionnez-le et faites-le glisser dans la vue.
browser-job-settings-problems-hover = Brut, machine, post-processeur et outils. { $count ->
        [one] { $count } problème
       *[other] { $count } problèmes
    }
browser-tools = Outils
browser-tools-hover = Les outils du projet : cliquer pour les voir tous et en ajouter un

# Parts: groups of operations, each cut with its copies before the next.
browser-parts = Pièces
browser-parts-hover = Chaque pièce est usinée avec ses copies avant la suivante ; clic droit pour une nouvelle pièce
browser-setups = Montages
browser-setups-hover = Les panneaux dans lesquels le projet est usiné, l'un après l'autre : chacun un programme à lui, avec son brut et son origine pièce. Un clic le rend actif
browser-add-setup = Ajouter un montage
browser-add-setup-hover = Un autre panneau : son brut, son zéro et ses pièces, usiné comme un programme à lui
browser-setup-parts = { $count ->
    [one] { $count } pièce
   *[other] { $count } pièces
}
browser-setup-active-hover = Le montage actif : affiché en couleur, simulé et calculé. Son brut et son origine sont dans Paramètres d'usinage et Origine pièce
browser-setup-hover = Un clic en fait le montage actif : affiché, simulé et calculé
browser-move-to-setup = Déplacer vers le montage
browser-new-part = Nouvelle pièce
# At a part row's right end: the part new operations go into.
browser-part-active = active
browser-part-active-hover = La pièce active : les nouvelles opérations y vont. Cliquer pour ses copies ; clic droit pour plus
browser-part-hover = Cliquer pour ses copies ; double-clic pour en faire la pièce active ; clic droit pour plus
# A verb: make this the active part.
browser-activate = Activer
browser-activate-hover = Les nouvelles opérations vont dans la pièce active
browser-export-part = Exporter le G-code de cette pièce…
browser-export-part-hover = Écrire le G-code de cette pièce seule, comme si toutes les autres pièces étaient désactivées ; sa fiche de réglage aussi si ce réglage est actif.
browser-new-operation = Nouvelle opération
browser-select-shapes-first = Sélectionnez d'abord des formes
browser-move-up = Monter
browser-move-down = Descendre
# $count: the part's operations.
browser-delete-part = { $count ->
        [0] Supprimer la pièce
        [one] Supprimer la pièce et son opération
       *[other] Supprimer la pièce et ses { $count } opérations
    }
browser-delete-part-disabled-hover = Un projet doit avoir au moins une pièce
browser-part-empty-active = Sélectionnez des formes, puis ajoutez une opération
browser-part-empty = Aucune opération : glissez-en une ici

# An operation's row and its right-click menu.
browser-no-name = (sans nom)
# $count: contours the operation cuts that are hidden on the canvas.
browser-op-hidden = { $count ->
        [one] { $count } masqué
       *[other] { $count } masqués
    }
browser-duplicate = Dupliquer
browser-add-rest-pass = Ajouter une passe de reprise
# A tool for a rest pass: $tool the project tool's number, $size its diameter.
browser-rest-project-tool = T{ $tool } · fraise { $size }
browser-rest-library-tool = { $name } · fraise { $size }
browser-from-library = Depuis votre bibliothèque d'outils
browser-copied-into-project-hover = Copiée dans les Outils du projet
browser-add-rest-pass-hover = Une passe avec un outil plus petit qui n'enlève que ce que l'outil de cette poche laisse, dans ses formes et profondeurs. Elle suit la poche, et est usinée après les poches voisines qui ont le même outil
browser-make-rest-pass = En faire une passe de reprise de l'opération { $number }
browser-make-rest-pass-hover = Elle copie cette poche : comme passe de reprise, elle la suit, et est usinée après les poches voisines qui ont le même outil
browser-delete = Supprimer

# A rest pass's row. "Rest": a rest pass, a smaller tool after the pocket's own.
browser-rest = Reprise
browser-rest-pass-hover = Une passe de reprise : elle enlève ce que l'outil précédent laisse, dans les formes et profondeurs de sa poche, et est usinée après les poches voisines de sa poche qui ont le même outil
browser-add-floor-pass = Ajouter une passe de fond
browser-add-floor-pass-hover = Une fraise droite qui dégage le fond entre les flancs des traits trop larges, dans les formes et profondeurs de cette gravure en V. Elle est usinée avant la gravure en V
browser-floor = Fond
browser-floor-pass-hover = Une passe de fond : une fraise droite dégage le fond entre les flancs de sa gravure en V, avant celle-ci ; la fraise en V reprend là où elle n'atteint pas
browser-remove = Retirer

# An operation row's hover text: its lines.
browser-op-rest-after = reprise après T{ $tool }
browser-op-not-cut = Non usinée (décochée)
browser-op-hidden-hover = Usine { $count } { $count ->
        [one] contour masqué
       *[other] contours masqués
    } : affichez-les dans Dessin pour les voir
browser-op-drag-hover = Glisser pour réordonner ; clic droit pour plus

# Properties: what is selected on the canvas.
browser-selection = Sélection
browser-copy-as-selector = Copier la sélection
browser-copy-as-selector-hover = La sélection en JSON, pour une opération dans un fichier projet

# A thin line between operation rows where the tool changes; $tool the new tool's number.
browser-tool-change = changement d'outil → T{ $tool }
