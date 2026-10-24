# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = bilden är { $bytes } byte; högst 20 MB stöds: skala ned den
image-error-too-many-pixels = bilden är { $width } × { $height } pixlar; högst 20 megapixlar stöds: skala ned den
image-error-format = inte en PNG-, JPEG- eller BMP-bild
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = bilden kan inte läsas: { $reason }
image-error-no-pixels = bilden har inga pixlar
image-error-decoder-failed = avkodaren misslyckades
# $zero is the length 0 with its unit.
image-error-width = bredden måste vara mer än { $zero }
image-error-too-many-loops = bilden spåras till { $loops } slingor, fler än { $most }: öka oskärpan, öka minsta bit eller ändra tröskeln

## Its settings

image-error-threshold = tröskeln måste vara 1 till 254, eller auto
image-error-blur = oskärpan måste vara 0 till 10 px
image-error-smallest-piece = minsta bit måste vara 0 till 100 mm²
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = utjämningen måste vara { $min } till { $max }
