# The property forms: an operation, the job settings, tools, DXFs, pictures, texts, parts (crates/chipload-gui/src/app/forms.rs).

# Depths, shared by the operations. "Through": a button that sets the end depth through the stock.
forms-start-depth = Start depth
forms-start-depth-hover = Where the cut starts, down from the stock top
forms-end-depth = End depth
forms-end-depth-hover = Where the cut ends, down from the stock top
forms-through = Through
forms-through-hover = Cut through the stock: { $depth }, its thickness plus the allowed depth below it (Job settings > Stock)
forms-oversize = Oversize
forms-oversize-disabled = A profile on the line has no side to cut more on
forms-step-down = Step down
# Indented under "Passes": the depth of each pass.
forms-step-down-sub = step down

# Cutting direction (milling).
forms-direction = Direction
forms-climb = climb
forms-conventional = conventional

# Tabs' sloped sides.
forms-flank-angle = flank angle
forms-flank-angle-hover = How steeply the tab's sides slope down from its flat top. 90° is a straight tab: the tool goes straight up and down. Below that the tool follows the slope, so it never plunges after a tab. Going down a flank is a ramp: with a tool that isn't centre-cutting, no steeper than the ramp angle.
forms-flank-straight = straight
forms-flank-feet = feet { $length } each side

# How the tool goes down into the material.
forms-entry = Entry
forms-entry-hover = How the tool goes down into the material. A ramp goes down along the path (a profile spirals down round its ring, a follow path goes down as it goes back and forth, a pocket ramps out and back), a helix (pockets) round a small circle, so the tool needn't be centre-cutting; both use the cutting feed.
forms-entry-plunge = straight plunge
forms-entry-ramp = ramp
forms-entry-helix = helix
forms-entry-ramp-summary = ramp { $angle }
forms-entry-helix-summary = helix r { $radius }, { $angle }
forms-entry-with-lead = { $entry }, lead { $radius }
forms-helix-radius = helix radius
forms-helix-radius-hover = Of the circle the tool centre follows: less than the tool radius, so no core is left standing
forms-ramp-angle-sub = ramp angle
forms-ramp-angle = Ramp angle
forms-ramp-angle-hover = The tool spirals down along the contour, never steeper than this and never more than one step down per lap: no plunge into the part. Each pass goes down at this angle, then runs level; a steeper angle makes the ramp shorter.

# The operation kinds, in the form's title.
forms-kind-profile = Profile
forms-kind-pocket = Pocket
forms-kind-vcarve = V-carve
forms-kind-path = Follow path
forms-kind-holes = Holes
forms-kind-surface = Surface
forms-operation-number = Operation { $number }

forms-name = Name
forms-geometry = Geometry
forms-whole-stock = The whole stock
forms-whole-stock-hover = A surface faces the whole stock, once, whatever the copies
forms-use-selection = Use selection
forms-use-selection-count = Use selection ({ $count })
forms-use-selection-hover = Replace with the contours selected on the canvas.
forms-use-selection-disabled = Select contours on the canvas first

# A profile's side of the line.
forms-side = Side
forms-side-inside = inside
forms-side-outside = outside
forms-side-on = on
forms-side-choose = — choose —

# Section titles and their folded summaries. "Cut": how the operation cuts (noun).
forms-section-cut = Cut
forms-profile-cut-summary = { $start }–{ $end } { $unit }, steps of { $step } { $unit }, { $direction }, spiral ramp { $ramp }°
forms-pocket-cut-summary = { $start }–{ $end } { $unit }, steps of { $step } { $unit }, { $direction }
forms-path-cut-summary = { $start }–{ $end } { $unit }, steps of { $step } { $unit }, on the line
forms-surface-cut-summary = { $start }–{ $end } { $unit }, steps of { $step } { $unit }, stepover { $stepover } × D
forms-vcarve-cut-one-pass = from { $start }, at most { $end } deep, one pass
forms-vcarve-cut-steps = from { $start }, at most { $end } deep, steps of { $step }

# A pocket's clearing, in rings.
forms-section-clearing = Clearing
forms-clearing-summary = rings, stepover { $stepover } × D
forms-clearing-summary-rest = rings, stepover { $stepover } × D, rest after T{ $tool }
forms-stepover = Stepover
forms-stepover-hover = How far each ring moves on, as a fraction of the tool diameter
forms-surface-stepover-hover = How far each pass moves across, as a fraction of the tool diameter
forms-surface-end-depth-hover = How much comes off the top, down from the stock top
forms-section-entry-lead = Entry and lead
forms-section-entry = Entry

# V-carving.
forms-vcarve-start-depth-hover = The surface carved, down from the stock top (0: the top; more to carve into a pocket floor)
forms-vcarve-end-depth-hover = The deepest the tip goes, from the stock top (end_depth in the file). Wider strokes are carved to it and reported
forms-passes = Passes
forms-in-steps = in steps

# The operation's tool. "T3" is tool number 3.
forms-tool = Tool
forms-tool-not-in-project = T{ $tool } (not in the project)
forms-from-library = From your library:
forms-use-cutting-values = Use the tool's cutting values
forms-use-cutting-values-hover = From the library: feed { $feed } / plunge { $plunge } { $feed_unit }, step down { $step_down }, stepover { $stepover }

forms-section-feeds = Feeds
forms-feed = Feed
forms-plunge-feed = Plunge feed

# Rest machining: clear only what a larger tool left.
forms-rest-after = Rest after
forms-rest-after-hover = Clear only what a larger tool leaves in these shapes: the corners it leaves round and the slots it doesn't fit in. Cut the pocket with that tool first
forms-rest-none = none
forms-rest-endmill = { $diameter } endmill

# Square inside corners.
forms-corners = Corners
forms-corners-hover = Inside corners a round tool leaves round: a dogbone goes into the corner along its middle, a T-bone along the longer wall (the notch is in the shorter one), so a square part fits all the way in
forms-corners-round = round
forms-corners-dogbones = dogbones
forms-corners-tbones = T-bones
forms-corners-disabled = A profile cut on the line has no inside corners
forms-corners-up-to = up to
forms-corners-up-to-hover = Only corners this sharp or sharper (90° corners drawn slightly off still count)
forms-corners-chosen = { $count ->
    [one] { $count } corner chosen on its own
   *[other] { $count } corners chosen one by one
}
forms-corners-chosen-hover = Corners clicked on the canvas and given their own choice, whatever the row above says
forms-corners-reset = Reset
forms-corners-reset-hover = Every corner gets what the row above says again

# Holes: drilled, or bored with an endmill round a helix.
forms-holes-drilled-pecks = drilled, pecks { $peck }
forms-holes-drilled-once = drilled in one go
forms-holes-bore = helical bore, { $step } per turn, { $direction }
forms-holes-cut-summary = { $start }–{ $end } { $unit }, { $method }
forms-holes-drill-end-depth-hover = Where the drill's point ends, down from the stock top: the full diameter stops about 0.3 × the drill's diameter higher
forms-pecks = Pecks
forms-pecks-hover = Drill this deep at a time, rising out of the hole in between to clear the chips
forms-in-pecks = in pecks
forms-peck = peck
forms-step-per-turn = Step per turn
forms-step-per-turn-hover = The most the helix goes down in one turn
forms-bore-ramp-angle-hover = The helix is never steeper than this
forms-drill-feeds-summary = { $feed } { $unit } down
forms-drill-feed = Drill feed

# How a profile holds its part.
forms-holding-none = none
forms-holding-tabs = tabs
forms-holding-skin = onion skin
forms-holding-skin-tabs = onion skin and tabs
forms-holding-summary-tabs = { $holding }: { $count } × { $width } { $unit }, { $height } { $unit } high
forms-holding-summary-skin = { $summary }, skin { $thickness }
forms-section-holding = Holding
forms-hold-the-part = Hold the part
forms-hold-the-part-hover =
    How a part cut out stays in place.
    Tabs: bridges left at the bottom of the cut; the part stays attached until you cut it free.
    Onion skin: a thin skin left all round during the roughing, then a last lap after every ring of the operation cuts through it. The part is free during that lap: add tabs if the parts must not move then.
forms-tab-count = tab count
forms-tab-width = width
forms-tab-height = height
forms-tabs-placed = { $count ->
    [one] { $count } tab placed by hand
   *[other] { $count } tabs placed by hand
}
forms-tabs-placed-hover = Tabs dragged, added or removed on the canvas; contours without any get the count above, spaced evenly
forms-tabs-reset = Reset
forms-tabs-reset-hover = Every contour's tabs spaced evenly again
forms-skin-thickness = skin thickness
forms-skin-thickness-hover = Left above the end depth until the last lap: at most one step down, and thinner than the tabs

# A profile's finishing pass.
forms-finish-none = none
forms-finish-summary = { $stock } left, then one lap
forms-finishing-pass = Finishing pass
forms-finishing-pass-hover = Rough with this much left on the wall, then cut it away in one light lap at full depth: a cleaner wall without step marks. The lap goes down in the roughing's slot, never on the finished wall.
forms-finishing-lap = finishing lap
forms-finishing-disabled = A profile cut on the line has no wall to finish
# The material left on the wall for the finishing lap.
forms-finish-stock = stock

# A pocket's lead-in and lead-out.
forms-lead = Lead
forms-lead-hover = Arc onto the wall from the cleared pocket and off it again, so the tool never stops against the wall. A lead without room shrinks, or is left out (the report says so).
forms-lead-in-out = lead-in and lead-out
forms-lead-radius = radius

# The G-code's X0 Y0.
forms-work-zero = Work zero X Y
forms-work-zero-hover = The point that becomes X0 Y0 in the G-code. The geometry doesn't move.
forms-zero-drawing-origin = drawing origin
forms-zero-stock-centre = stock centre
forms-zero-stock-bottom-left = stock bottom left
forms-zero-stock-bottom-right = stock bottom right
forms-zero-stock-top-left = stock top left
forms-zero-stock-top-right = stock top right
forms-zero-point = point

# An imported DXF.
forms-file = File
forms-units = Units
forms-dxf-inch = inch (converted to mm)
forms-shapes = Shapes
forms-section-placement = Position, rotation and scale
forms-update-dxf = Update from a DXF…
forms-update-dxf-hover = A new version of the drawing: operations keep their layers
# A button (verb).
forms-remove = Remove
forms-remove-dxf-hover = Remove this DXF and its shapes

# A picture traced into shapes.
forms-picture = Picture
forms-picture-pixels = { $width } × { $height } pixels
forms-traced-as = Traced as
forms-traced-outlines = outlines
forms-traced-centre-lines = centre lines
forms-position = Position
forms-picture-reset-position = The box's corner at X0 Y0
forms-rotation = Rotation
forms-picture-rotation-hover = Degrees counter-clockwise, any angle, about the picture's centre. The picture is traced as it is, then its shapes are turned.
forms-turn-ccw-hover = 90° counter-clockwise
forms-turn-cw-hover = 90° clockwise
forms-picture-reset-rotation = As the picture is (0°)
forms-width = Width
forms-picture-width-hover = The picture's width before it is turned; the height follows
forms-picture-height = height { $height }
forms-layer = Layer
forms-picture-layer-hover = The layer its shapes are on, for operations that select it
forms-trace-settings = Trace settings…
forms-trace-settings-hover = Threshold, smoothing, outlines or centre lines, with a preview
forms-remove-picture-hover = Remove the picture and its shapes

# A text written in a built-in font.
forms-text = Text
forms-text-hover = What it says; Enter starts a new line
forms-font = Font
forms-font-hover = A built-in font: plain, script or display (User guide: Text)
forms-height = Height
forms-text-height-hover = The height of a capital letter
forms-anchor = Anchor
forms-anchor-hover = What Position is: the lower-left corner of the text's box, or its centre. A centred text stays centred when its words, font or size change
forms-anchor-corner = lower-left corner
forms-anchor-centre = centre
forms-text-reset-position = At X0 Y0
forms-text-rotation-hover = Degrees counter-clockwise, any angle, about the text's centre
forms-text-reset-rotation = Level (0°)
forms-align = Align
forms-align-hover = How the lines line up against each other
forms-align-left = left
forms-align-centre = centre
forms-align-right = right
forms-letter-spacing = Letter spacing
forms-letter-spacing-hover = Extra space between letters; 0: the font's own
forms-line-spacing = Line spacing
forms-line-spacing-hover = From one line's base to the next, in capital heights
# After the number: "1.2 × height".
forms-line-spacing-suffix = × height
forms-text-layer-hover = The layer its letters are on, for operations that select it
forms-size = Size
forms-remove-text-hover = Remove the text and its letters

# The job settings. "Stock": the material the parts are cut from.
forms-job-settings = Job settings
forms-section-stock = Stock
forms-stock-size = Size X × Y
forms-thickness = Thickness
forms-max-overcut = Max depth below stock
forms-max-overcut-hover = How far a cut may go below the stock (into the spoilboard). Cuts within it are warned about; deeper ones are refused.
forms-stock-position = Stock position
forms-stock-corner-at-origin = corner at drawing origin
forms-stock-centred-on-origin = centred on drawing origin
forms-z-zero = Z zero
forms-z-zero-top = stock top
forms-z-zero-bottom = stock bottom
forms-machine-summary = { $post }, safe Z { $safe_z }, { $spindle }
forms-section-machine = Machine
forms-from-machine = From machine
forms-edit-here = Edit here
forms-edit-here-hover = Change the machine's values for this project only; it stays linked, and shows what differs
forms-safe-z-job = Safe Z (this job)
forms-safe-z-hover = The height for rapid moves sideways, above the stock and its clamps

# The project's tools.
forms-tools = Tools
forms-add-from-library = Add from library…
forms-new-tool = New tool
forms-tool-not-used = not used
# $operations: a list of operation numbers, "1, 3".
forms-tool-used-by = operations { $operations }
forms-tool-not-used-long = Not used by an operation
forms-tool-cuts = Cuts operations { $operations }
forms-remove-tool = Remove tool
forms-remove-tool-disabled = Used by operations { $operations }; give them another tool first
# An icon button that puts a value back.
forms-reset = Reset

# A DXF's position, rotation and scale.
forms-dxf-reset-position = Back to where the DXF has it (turn and scale stay)
forms-bottom-left = Bottom left
forms-bottom-left-hover = The drawing's corner { $margin } in from the stock's lower-left corner
# A button: centre the drawing on the stock.
forms-centre = Centre
forms-centre-hover = The drawing's centre on the stock's centre
forms-dxf-rotation-hover = Degrees counter-clockwise from the DXF as drawn, any angle, about the drawing's centre
forms-dxf-reset-rotation = Back to the DXF's own angle (0°)
forms-scale = Scale
forms-scale-hover = From the DXF as drawn, about the drawing's centre. Tool sizes, depths and tabs don't scale.
forms-dxf-reset-scale = Back to the DXF's own size (× 1)

# A part: a group of operations cut one after the other.
forms-part-number = Part { $number }
# Label of the on/off checkbox (verb): whether the part is cut.
forms-part-cut = Cut
forms-part-cut-hover = Off: the part and its operations stay in the project, not cut
forms-part-cut-check = this part and its operations
forms-active = Active
forms-active-hover = New operations go into the active part. Right-click a part to activate it
forms-active-yes = yes: new operations go here
forms-activate = Activate
forms-operations = Operations
forms-operations-none = none yet
forms-copies-one = one
forms-copies = Copies
forms-parts-note = Parts are cut one after the other, each with its copies.
forms-copies-hover = Cut the part several times in a grid. The copies are drawn faded; only the drawing itself can be selected.
forms-copies-grid = several, in a grid
forms-columns-rows = columns × rows
# Copy to copy distance, in X and Y.
forms-pitch = pitch X Y
forms-gap = gap
forms-gap-hover = Sets the pitch once so this much is left between the cuts of neighbouring copies. The project keeps the pitch.
forms-set-pitch = Set pitch
forms-order = order

# The machine. "Post": the post-processor that writes the G-code.
forms-post = Post
forms-post-yours = { $name } (yours)
forms-post-folder-hover = Your own post-processors go in { $folder }
forms-post-builtin-hover = The built-in post-processors
forms-gcode-units = G-code units
forms-gcode-units-hover = The units the G-code is written in: mm (G21) or inches (G20), as your controller is set. Everything else in the project stays in mm
forms-gcode-mm = mm (G21)
forms-gcode-inch = inch (G20)
forms-spindle = Spindle
forms-spindle-cw = cw (M3)
forms-spindle-ccw = ccw (M4)
forms-max-feed = Max feed
forms-tool-changes = Tool changes
forms-tool-changes-hover = With several tools: one G-code file per tool (change the tool and set Z zero between files), or one file with M6 tool changes that your sender handles (GRBL itself refuses M6)
forms-tool-change-files = one file per tool
forms-tool-change-program = one file with M6 (your sender handles the change)
forms-travel = Travel X Y Z
forms-max-feed-not-set = not set
forms-travel-not-checked = not checked
forms-revert-hover = "{ $name }" has { $value }: click to take it
forms-post-yours-copied = Yours, copied into the project
forms-post-changed = Your post file has changed
forms-post-update = Update
forms-post-update-hover = Copy the post as it is now into the project
forms-post-file-missing = Yours; the project keeps its copy (the file isn't there)

# A tool's values.
forms-tool-type = Type
forms-diameter = Diameter
forms-spindle-speed = Spindle speed
forms-centre-cutting = Centre cutting
forms-yes = yes
forms-no = no
forms-unknown = unknown
forms-angle = Angle
forms-angle-hover = The included angle of the V; V-carving needs it
forms-tip = Tip
forms-tip-hover = The diameter of the flat tip; not set is taken as pointed
forms-pointed = pointed

# A rest pass of a pocket: a smaller tool clears what the one before left.
forms-rest-pass-of = operation { $number }
forms-rest-pass-of-named = operation { $number } ({ $name })
forms-rest-pass-facts = { $start }–{ $end } { $unit }, clearance { $clearance } { $unit }, { $direction }
forms-rest-pass = Rest pass
forms-rest-of = Rest of
forms-rest-of-hover = It cuts its pocket's shapes and depths, and clears only what the tool before it leaves
# A button: show the pocket.
forms-open = Open
forms-from-pocket = From the pocket
forms-after-tool = After tool
forms-after-tool-hover = It clears what this tool leaves: the pocket's, or the pass before
forms-cut-it = Cut it
forms-cut-it-hover = Off: kept, but not cut
forms-rest-pass-note = Cut after the pockets next to its pocket that use the same tool, all their rest passes together: the small tool comes in once.
forms-remove-rest-pass = Remove rest pass

# Font styles, headings in the font list.
forms-font-plain = Plain
forms-font-script = Script
forms-font-display = Display
forms-explode-dxf = Explode…
forms-explode-dxf-hover = Turn this DXF's shapes into drawn shapes you can edit; the link to the DXF file goes
