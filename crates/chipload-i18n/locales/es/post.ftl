# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = Post-processor config schema_version { $found } is not supported (this app supports up to { $current }). Update the application or check the file.
post-error-cannot-read = Cannot read post-processor config: { $error }
post-error-cannot-read-file = Cannot read post-processor config { $path }: { $error }
# $path is a config or template file.
post-error-cannot-read-path = Cannot read { $path }: { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Invalid post-processor config: { $error }
post-error-template-name = template must be a plain file name, got { $got }
post-error-decimals = decimals must be at most 6, got { $got }
post-error-inch-decimals = inch_decimals must be at most 6, got { $got }
post-error-units-empty = units must name at least one of mm and inch
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s must be at least 0, got { $got }
post-error-template-unreadable = Template { $template } can't be read: { $error }
post-error-name = Invalid post-processor name { $name }
post-error-builtin-name = { $name } is the name of a built-in post-processor; rename your file
post-error-unknown = Unknown post-processor { $name }
post-error-unknown-template = Unknown template { $template }
post-error-syntax = Template syntax settings: { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = Post-processor { $post } can't write G-code in { $units }: its config doesn't list { $listed } under units (its template must write { "{{ units_word }}" } instead of G21)
post-error-tool-changes = Post-processor { $post } can't write tool changes in one file; write one file per tool instead (tool changes: files)
post-error-arc-centre = Arc move without a centre
post-error-arc-from = Arc from an unknown position
post-error-arc-to = Arc to an unknown position
post-error-feed-rate = Feed move without a feed rate
post-error-no-groups = No tool groups to render
post-error-template-failed = Template { $template } failed: { $error }
