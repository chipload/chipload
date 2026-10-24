# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses.

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = le post-processeur { $post } n'écrit que du G-code en mm (sa configuration ne liste pas "{ $units }" sous units)
