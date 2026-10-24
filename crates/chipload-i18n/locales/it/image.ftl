# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = l'immagine è di { $bytes } byte; sono supportati al massimo 20 MB: riducila
image-error-too-many-pixels = l'immagine è di { $width } × { $height } pixel; sono supportati al massimo 20 megapixel: riducila
image-error-format = non è un'immagine PNG, JPEG o BMP
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = impossibile leggere l'immagine: { $reason }
image-error-no-pixels = l'immagine non ha pixel
image-error-decoder-failed = il decodificatore non è riuscito
# $zero is the length 0 with its unit.
image-error-width = la larghezza deve essere più di { $zero }
image-error-too-many-loops = l'immagine si vettorizza in { $loops } anelli, più di { $most }: aumenta la sfocatura, aumenta il pezzetto minimo o cambia la soglia

## Its settings

image-error-threshold = la soglia deve essere da 1 a 254, o auto
image-error-blur = la sfocatura deve essere da 0 a 10 px
image-error-smallest-piece = il pezzetto minimo deve essere da 0 a 100 mm²
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = la levigatura deve essere da { $min } a { $max }
