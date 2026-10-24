# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = la imagen ocupa { $bytes } bytes; se admiten como máximo 20 MB: redúcela
image-error-too-many-pixels = la imagen mide { $width } × { $height } píxeles; se admiten como máximo 20 megapíxeles: redúcela
image-error-format = no es una imagen PNG, JPEG ni BMP
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = no se puede leer la imagen: { $reason }
image-error-no-pixels = la imagen no tiene píxeles
image-error-decoder-failed = el decodificador falló
# $zero is the length 0 with its unit.
image-error-width = el ancho debe ser más de { $zero }
image-error-too-many-loops = la imagen se vectoriza en { $loops } lazos, más de { $most }: desenfócala más, sube la mancha mínima o cambia el umbral

## Its settings

image-error-threshold = el umbral debe ser de 1 a 254, o auto
image-error-blur = el desenfoque debe ser de 0 a 10 px
image-error-smallest-piece = la mancha mínima debe ser de 0 a 100 mm²
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = el suavizado debe ser de { $min } a { $max }
