# SVG import (docs/specs/svg-import.md).
svg-error-too-large = the SVG is larger than { $most }
# $reason is the parser's own words, in English.
svg-error-unreadable = not an SVG that can be read: { $reason }
svg-error-crashed = the SVG parser crashed on this file
svg-error-too-deep = elements are nested deeper than { $most }
svg-error-needs-scale = the SVG gives no real size (only pixels): choose its pixels per inch, 96 (Inkscape, browsers) or 72 (older Illustrator)
svg-error-scale = the scale or pixels per inch must be more than 0, got { $got }
svg-error-too-many = the SVG has { $count } shapes, more than { $most }
svg-error-far-out = { $element } has a point that is not a number or lies more than { $most } from the origin
