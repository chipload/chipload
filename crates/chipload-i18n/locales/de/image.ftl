# Messages of the picture tracer (crates/chipload-image): why a picture can't be read or
# traced. A length argument brings its own unit ("1 mm").

## The picture

# $bytes is the file's size in bytes.
image-error-too-large = the picture is { $bytes } bytes; at most 20 MB is supported: scale it down
image-error-too-many-pixels = the picture is { $width } × { $height } pixels; at most 20 megapixels is supported: scale it down
image-error-format = not a PNG, JPEG or BMP picture
# $reason is the picture decoder's own words, or one of the two texts below.
image-error-decode = the picture can't be read: { $reason }
image-error-no-pixels = the picture has no pixels
image-error-decoder-failed = the decoder failed
# $zero is the length 0 with its unit.
image-error-width = the width must be more than { $zero }
image-error-too-many-loops = the picture traces into { $loops } loops, more than { $most }: blur it more, raise the smallest piece, or change the threshold

## Its settings

image-error-threshold = the threshold must be 1 to 254, or auto
image-error-blur = the blur must be 0 to 10 px
image-error-smallest-piece = the smallest piece must be 0 to 100 mm²
# $min is a length without its unit, $max one with it: "0.01 to 1 mm".
image-error-smoothing = the smoothing must be { $min } to { $max }
