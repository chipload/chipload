# Messages of the app services (crates/chipload-pipeline): problems found with the
# post-processor a project uses.

# $post is the post-processor's name, $units the G-code units the machine is set to, as
# written in the project file ("inch"). "units" is a key of the post's config file.
pipeline-post-units = postprocessorn { $post } skriver bara G-kod i mm (dess konfiguration har inte "{ $units }" under units)
