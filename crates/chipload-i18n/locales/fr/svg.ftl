# Import SVG (docs/specs/svg-import.md).
svg-error-too-large = le SVG dépasse { $most }
svg-error-unreadable = ce n'est pas un SVG lisible : { $reason }
svg-error-crashed = l'analyseur SVG a planté sur ce fichier
svg-error-too-deep = des éléments sont imbriqués sur plus de { $most } niveaux
svg-error-needs-scale = le SVG ne donne pas de taille réelle (seulement des pixels) : choisissez ses pixels par pouce, 96 (Inkscape, navigateurs) ou 72 (anciens Illustrator)
svg-error-scale = l'échelle ou les pixels par pouce doivent être supérieurs à 0, reçu { $got }
svg-error-too-many = le SVG a { $count } formes, plus que { $most }
svg-error-far-out = { $element } a un point qui n'est pas un nombre ou se trouve à plus de { $most } de l'origine
svg-px-chosen = le SVG ne donne pas de taille réelle ; lu à { $px } pixels par pouce, comme choisi
svg-text-left-out = { $count ->
    [one] 1 texte laissé de côté
   *[other] { $count } textes laissés de côté
} : convertissez le texte en chemins dans son éditeur (Inkscape : Chemin > Objet en chemin ; Illustrator : Vectoriser le texte) ou utilisez le texte de Chipload
svg-images-left-out = { $count ->
    [one] 1 image laissée de côté
   *[other] { $count } images laissées de côté
} ; une image à vectoriser s'ajoute à part
svg-clips-ignored = { $count ->
    [one] 1 tracé de découpe ou masque ignoré
   *[other] { $count } tracés de découpe ou masques ignorés
} : ce qu'ils découpent est importé en entier
svg-hidden-paths = { $count ->
    [one] 1 chemin invisible dans le fichier (sans remplissage ni contour, ou masqué) a été importé
   *[other] { $count } chemins invisibles dans le fichier (sans remplissage ni contour, ou masqués) ont été importés
}
svg-empty-dropped = { $count ->
    [one] 1 sous-chemin sans longueur laissé de côté
   *[other] { $count } sous-chemins sans longueur laissés de côté
}
