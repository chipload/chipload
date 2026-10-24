# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = das Bild hat { $bytes } Byte; höchstens 20 MB werden unterstützt: verkleinern Sie es
image-error-too-many-pixels = das Bild hat { $width } × { $height } Pixel; höchstens 20 Megapixel werden unterstützt: verkleinern Sie es
image-error-format = kein PNG-, JPEG- oder BMP-Bild
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = das Bild kann nicht gelesen werden: { $reason }
image-error-no-pixels = das Bild hat keine Pixel
image-error-decoder-failed = der Decoder ist fehlgeschlagen
# $zero is the length 0 with its unit.
image-error-width = die Breite muss mehr als { $zero } sein
image-error-too-many-loops = das Bild ergibt beim Nachzeichnen { $loops } Schleifen, mehr als { $most }: zeichnen Sie es stärker weich, vergrößern Sie das kleinste Stück oder ändern Sie den Schwellwert

## Its settings

image-error-threshold = der Schwellwert muss 1 bis 254 oder auto sein
image-error-blur = das Weichzeichnen muss 0 bis 10 px sein
image-error-smallest-piece = das kleinste Stück muss 0 bis 100 mm² sein
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = die Glättung muss { $min } bis { $max } sein
