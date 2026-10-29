# The setup sheet for the operator (crates/chipload-pipeline/src/sheet.rs,
# docs/specs/setup-sheet.md): an HTML page printed next to the machine. A length argument
# brings its own unit ("6 mm", "0.25 in"); $start, $end, $x, $y, $width, $step, $radius and
# $feed are numbers alone, in the sheet's units, as are $z and $peck where said. Names
# (project, tools, parts, operations, layers, files) are never translated.

sheet-title = Setup sheet: { $project }
# Only some of the project's parts were exported.
sheet-title-parts = Setup sheet: { $project }, { $count ->
    [one] part
   *[other] parts
} { $parts }
# The line under the title, its parts joined with " · ".
sheet-post = post { $post }
sheet-written-by = written by { $by }
# $length is "mm" or "in", $feed "mm/min" or "in/min".
sheet-units = lengths in { $length }, feeds in { $feed }
sheet-gcode-inch = G-code in inches (G20)

## Stock and zero
sheet-stock-and-zero = Stock and zero
sheet-stock = Stock
# This text is HTML: keep &times; (the × sign). $width is a number alone.
sheet-stock-size = { $width } &times; { $length }, { $thickness } thick
sheet-copies = Copies
sheet-part = Part
sheet-dxf = DXF { $id }
# $placement says how the drawing was moved, turned and scaled.
sheet-dxf-placed = { $name } ({ $file }), { $placement }
sheet-image = Image { $id }
sheet-image-placed = { $file } on layer { $layer }, { $width } wide at X{ $x } Y{ $y }
sheet-text = Text { $id }
# $text is the text's first words in quotes, $font its font.
sheet-text-placed = { $text } in { $font } on layer { $layer }, { $height } high at X{ $x } Y{ $y }

## Where X0 Y0 is, said on the stock
sheet-zero-centre = the centre of the stock
sheet-zero-bottom-left = the stock's bottom left corner
sheet-zero-bottom-right = the stock's bottom right corner
sheet-zero-top-left = the stock's top left corner
sheet-zero-top-right = the stock's top right corner
# A zero away from the stock's bottom left corner: "25 mm right of and 10 mm above the
# stock's bottom left corner". $offset is one or two of the four below, joined with -and.
sheet-zero-from-corner = { $offset } the stock's bottom left corner
sheet-zero-right-of = { $distance } right of
sheet-zero-left-of = { $distance } left of
sheet-zero-above = { $distance } above
sheet-zero-below = { $distance } below
sheet-zero-and = { $first } and { $second }
sheet-zero-outside = { $zero } (outside the stock)
# Where Z0 is.
sheet-z-top = the top of the stock
sheet-z-bottom = the bottom of the stock (the spoilboard)

## A file's tools. $what is the tool: "6 mm end mill, 2 flutes".
# $xy is where X0 Y0 is, $z where Z0 is (texts above).
sheet-load-tool = Load tool T{ $tool } ({ $what }). Set X0 Y0 at { $xy } and Z0 on { $z }.
sheet-tool-change-m6 = Tool change (M6) to T{ $tool } ({ $what }), as your controller handles it.
sheet-change-tool = Change to tool T{ $tool } ({ $what }). Set Z0 again with this tool on { $z }; keep X0 Y0.
# These two texts are HTML: keep &ldquo; and &rdquo; (the quotes round the tool's name).
# $rpm is the spindle speed, $spindle one of the two -spindle- texts.
sheet-tool-named = Tool T{ $tool }: &ldquo;{ $name }&rdquo;, { $what }, { $rpm } rpm { $spindle }.
sheet-tool = Tool T{ $tool }: { $what }, { $rpm } rpm { $spindle }.
sheet-spindle-cw = clockwise (M3)
sheet-spindle-ccw = counter-clockwise (M4)
# The parts of $what, joined with ", ".
sheet-end-mill = { $diameter } end mill
sheet-drill = { $diameter } drill
# $angle in degrees.
sheet-vbit-angle = { $angle }° V-bit, { $diameter }
sheet-vbit = V-bit, { $diameter }
sheet-tip = tip { $diameter }
sheet-flutes = { $count ->
    [one] { $count } flute
   *[other] { $count } flutes
}

## The table of operations
sheet-column-operation = Operation
sheet-column-depth = Depth
sheet-column-feeds = Feed / plunge
sheet-column-entry = Entry
sheet-column-tabs = Tabs
sheet-column-time = Time
# A part's row over its operations, numbered from 1.
sheet-part-row = Part { $number }: { $name }
sheet-part-row-copies = Part { $number }: { $name }, { $count } copies
# An operation without a name.
sheet-operation = Operation { $number }
# A drill's one feed: $feed is a number alone.
sheet-feed-down = { $feed } down
# The kind column, its parts joined with ", ": "Profile, outside, climb, leaves 0.2 mm".
sheet-kind-profile = Profile
sheet-side-outside = outside
sheet-side-inside = inside
sheet-side-on = on the line
sheet-kind-pocket = Pocket, rings
sheet-climb = climb
sheet-conventional = conventional
# $percent: the stepover in percent of the tool's diameter.
sheet-stepover = stepover { $percent } %
sheet-kind-vcarve = V-carve
sheet-kind-path = Follow path, on the line
sheet-kind-holes-drilled = Holes, drilled
sheet-kind-holes-bored = Holes, helical bore
sheet-kind-surface = Surface, the whole stock, back and forth (climb and conventional), stepover { $percent } %
# $surface is the text above; $distance half the tool's diameter.
sheet-kind-surface-past = { $surface }; runs { $distance } past the stock ends: keep clamps clear
sheet-cuts-past = cuts { $distance } past the line
sheet-leaves = leaves { $distance }
sheet-rest-after = rest after T{ $tool }
sheet-rest-after-size = rest after T{ $tool } ({ $diameter })
# $degrees: the sharpest corner angle that gets one.
sheet-dogbones = dogbones at corners up to { $degrees }°
sheet-tbones = T-bones at corners up to { $degrees }°
sheet-corner-choices = { $count ->
    [one] { $count } corner chosen on its own
   *[other] { $count } corners chosen one by one
}
# The depth column, its parts joined with ", ": "0 to 6, 2 per pass".
sheet-depths = { $start } to { $end }
sheet-depths-drill = { $start } to { $end } (the drill's point)
sheet-pecks = pecks { $peck }
sheet-one-go = in one go
sheet-per-turn = { $step } per turn
sheet-per-pass = { $step } per pass
sheet-one-pass = one pass
sheet-finish = finish { $stock }
sheet-skin = skin { $thickness }
# The entry column.
sheet-entry-beside = down beside the stock
sheet-entry-straight = straight down
sheet-entry-plunge = plunge
sheet-entry-ramp = ramp { $degrees }°
sheet-entry-helix = helix { $degrees }°
sheet-entry-helix-radius = helix r { $radius }, { $degrees }°
sheet-lead = lead r { $radius }
# The tabs column: $count tabs, $width and $height numbers alone.
sheet-tabs = { $count } × { $width } wide, { $height } high
sheet-flanks = flanks { $degrees }°
sheet-tabs-none = none
# Under the table: how deep the tool goes. $z is a number alone; the parts are joined with ", ".
sheet-cuts-down = Cuts down to Z{ $z }: { $depth } below the stock top
sheet-into-spoilboard = { $depth } into the spoilboard
sheet-through = through the stock
sheet-sticks-out = { $cut }. Check that the tool sticks out far enough.

## Watch out: the notes and the checker's findings
sheet-watch-out = Watch out
sheet-nothing-reported = Nothing was reported.
# $note is the post-processor's own note, as its file writes it.
sheet-post-note = Post { $post }: { $note }
sheet-check = Check: { $finding }

## The picture's legend
sheet-legend-file = File { $number } (T{ $tool })
sheet-legend-tabs = tabs
sheet-legend-grey = Grey: the parts. Cutting moves only, no rapids.

## The time
sheet-time = Time
