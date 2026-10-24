# The project's problems shown while editing: the Problems strip at the top of the
# properties panel, and the names of the fields they are about
# (crates/chipload-gui/src/app/problems.rs).

# The names of fields, put before a problem: "Feed: must be greater than 0, got 0".
# "Stock" is the material being machined.
problems-field-stock-x = Stock X
problems-field-stock-y = Stock Y
problems-field-thickness = Thickness
problems-field-max-overcut = Max depth below stock
problems-field-safe-z = Safe Z
# The machine's travel: how far its axes can move.
problems-field-travel = Travel
problems-field-copies = Copies
problems-field-part-name = Part name
problems-field-parts = Parts
problems-field-tool-diameter = Tool diameter
problems-field-spindle-speed = Spindle speed
problems-field-vbit-angle = V-bit angle
# A V-bit's tip diameter.
problems-field-tip = Tip
problems-field-feed = Feed
problems-field-plunge-feed = Plunge feed
problems-field-start-depth = Start depth
problems-field-end-depth = End depth
problems-field-step-down = Step down
problems-field-stepover = Stepover
problems-field-clearance = Clearance
problems-field-entry = Entry
problems-field-ramp-angle = Ramp angle
problems-field-lead = Lead
problems-field-tool = Tool
problems-field-part = Part
# The side of the line a profile cuts on.
problems-field-side = Side
# The shapes an operation cuts.
problems-field-geometry = Geometry
problems-field-tabs = Tabs
# $field: one of the names above; $problem: the problem's own words.
problems-with-field = { $field }: { $problem }

# The heading of the Problems strip.
problems-count = { $count ->
        [one] { $count } problem
       *[other] { $count } problems
    }
problems-updates-when-fixed = The toolpath updates once they are fixed.
# Where a problem is: in operation 2, or in the job's settings.
problems-place-operation = Operation { $number }
problems-place-job = Job
# One line of the strip: $place is one of the two above.
problems-line = { $place } · { $problem }
problems-show-it-hover = Show it
