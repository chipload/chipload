# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = l'image fait { $bytes } octets ; 20 Mo au plus sont acceptés : réduisez-la
image-error-too-many-pixels = l'image fait { $width } × { $height } pixels ; 20 mégapixels au plus sont acceptés : réduisez-la
image-error-format = ce n'est pas une image PNG, JPEG ou BMP
image-error-heic = une photo HEIC, que Chipload ne lit pas : enregistrez d'abord la carte de profondeur (ou la photo) en PNG ; pour un objet, le STL d'une application de scan 3D convient mieux
image-error-depth-setting = { $field } doit être entre { $low } et { $high }, vaut { $got }
image-error-depth-empty = toute l'image est du fond : baissez le niveau du fond
image-error-inflate-empty = l'image n'a aucune forme à ce seuil : changez le seuil, ou inversez-la
image-error-inflate-reach = la portée doit être supérieure à 0, vaut { $got }
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = impossible de lire l'image : { $reason }
image-error-no-pixels = l'image n'a aucun pixel
image-error-decoder-failed = le décodeur a échoué
# $zero is the length 0 with its unit.
image-error-width = la largeur doit être supérieure à { $zero }
image-error-too-many-loops = l'image se vectorise en { $loops } boucles, plus de { $most } : augmentez le flou ou le plus petit morceau, ou changez le seuil

## Its settings

image-error-threshold = le seuil doit être de 1 à 254, ou auto
image-error-blur = le flou doit être de 0 à 10 px
image-error-smallest-piece = le plus petit morceau doit être de 0 à 100 mm²
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = le lissage doit être de { $min } à { $max }
