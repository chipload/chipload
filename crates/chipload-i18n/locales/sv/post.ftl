# Messages of the post-processors (crates/chipload-post): a config or template that can't
# be read, or a program it can't write. Names of posts, templates and the config's keys
# (schema_version, decimals, units, …) are written as in the config file and never
# translated; $name, $post and $template come in quotes. $error is the words of the
# library that failed (the TOML reader, the template engine, the file system).

## Reading a config

post-error-version = Postprocessorkonfigurationens schema_version { $found } stöds inte (den här appen stöder upp till { $current }). Uppdatera programmet eller kontrollera filen.
post-error-cannot-read = Kan inte läsa postprocessorkonfigurationen: { $error }
post-error-cannot-read-file = Kan inte läsa postprocessorkonfigurationen { $path }: { $error }
# $path is a config or template file.
post-error-cannot-read-path = Kan inte läsa { $path }: { $error }
# $error is the TOML reader's words, or the problems below, joined by semicolons.
post-error-invalid = Ogiltig postprocessorkonfiguration: { $error }
post-error-template-name = template måste vara ett enkelt filnamn, fick { $got }
post-error-decimals = decimals får vara högst 6, fick { $got }
post-error-inch-decimals = inch_decimals får vara högst 6, fick { $got }
post-error-units-empty = units måste ange minst en av mm och inch
# $got is a time in seconds, without its unit.
post-error-dwell = spindle_dwell_s måste vara minst 0, fick { $got }
post-error-template-unreadable = Mallen { $template } kan inte läsas: { $error }
post-error-name = Ogiltigt postprocessornamn { $name }
post-error-builtin-name = { $name } är namnet på en inbyggd postprocessor; byt namn på din fil
post-error-unknown = Okänd postprocessor { $name }
post-error-unknown-template = Okänd mall { $template }
post-error-syntax = Mallens syntaxinställningar: { $error }

## Writing a program

# $units is "mm" or "inch", $listed the same in quotes, as the config would list it.
# "{{ units_word }}" and G21 are written in the template that way.
post-error-units = Postprocessorn { $post } kan inte skriva G-kod i { $units }: dess konfiguration har inte { $listed } under units (dess mall måste skriva { "{{ units_word }}" } i stället för G21)
post-error-tool-changes = Postprocessorn { $post } kan inte skriva verktygsbyten i en fil; skriv en fil per verktyg i stället (Verktygsbyten: files)
post-error-arc-centre = Bågrörelse utan centrum
post-error-arc-from = Båge från en okänd position
post-error-arc-to = Båge till en okänd position
post-error-feed-rate = Matningsrörelse utan matningshastighet
post-error-no-groups = Inga verktygsgrupper att skriva ut
post-error-template-failed = Mallen { $template } misslyckades: { $error }
