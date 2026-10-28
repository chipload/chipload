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
svg-px-chosen = the SVG gives no real size; read at { $px } pixels per inch, as chosen
svg-text-left-out = { $count ->
    [one] 1 text left out
   *[other] { $count } texts left out
}: turn text into paths in its editor (Inkscape: Path > Object to Path; Illustrator: Create Outlines), or use Chipload's own text
svg-images-left-out = { $count ->
    [one] 1 picture left out
   *[other] { $count } pictures left out
}; a picture to trace is added on its own
svg-clips-ignored = { $count ->
    [one] 1 clip path or mask ignored
   *[other] { $count } clip paths or masks ignored
}: what they clip is imported whole
svg-hidden-paths = { $count ->
    [one] 1 path that doesn't show in the file (no fill and no stroke, or hidden) was imported
   *[other] { $count } paths that don't show in the file (no fill and no stroke, or hidden) were imported
}
svg-empty-dropped = { $count ->
    [one] 1 subpath with no length left out
   *[other] { $count } subpaths with no length left out
}
