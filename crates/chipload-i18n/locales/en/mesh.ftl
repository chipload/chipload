# Messages of the STL reader (crates/chipload-mesh): why a 3D model can't be read.

# $bytes is the file's size in bytes.
mesh-error-too-large = the model is { $bytes } bytes; at most 300 MB is supported
mesh-error-too-many-triangles = the model has { $count } triangles; at most 5 million are supported: simplify it
# $expected is the size a binary STL with its triangle count would have, $found the file's size, both in bytes.
mesh-error-not-stl = not an STL file: a binary STL of its count would be { $expected } bytes, and it is { $found }, and it doesn't read as text either
mesh-error-not-a-number = the model has a corner that isn't a number
# $line is a line number in the text STL file.
mesh-error-text = the text STL breaks off at line { $line }
mesh-error-empty = the model has no triangles with an area
