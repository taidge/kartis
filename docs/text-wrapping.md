# Measured automatic text wrapping

[简体中文](text-wrapping.zh-CN.md)

These APIs are available in Kartis 0.2.0.
Wrapping is a Rust extension over Nivo's ordinary single-line SVG/Canvas text;
it does not implement a full CSS text layout or UAX #14 line-breaking engine.

`wrap_label` accepts a label, resolved font, `TextWrapOptions`, and a borrowed
real-measurement callback. `wrap_scene_label` replaces one Scene label only
after measuring and validating the entire candidate. Both return
`WrappedLabelLines`, including each ordinary drawable label, its bound
measurement, and whether the selected overflow policy truncated the source.

With the `raster` feature, `FontRegistry::wrap_label` and
`FontRegistry::wrap_scene_label` provide those measurements using actual
OpenType shaping and font bytes. Their final argument is `MissingGlyphPolicy`.
For example, after registering and binding the supplied font faces:

```rust
# use kartis::*;
# use kartis::render::raster::*;
# fn apply(scene: &mut Scene, fonts: &FontRegistry) -> Result<()> {
let options = TextWrapOptions {
    max_width: 120.,
    max_lines: 4,
    overflow: TextWrapOverflow::Ellipsis,
    ..Default::default()
};
let lines = fonts.wrap_scene_label(scene, 0, &options, MissingGlyphPolicy::Error)?;
assert!(lines.labels.len() <= 4);
# Ok(())
# }
```

Word and grapheme modes use complete Unicode extended grapheme clusters.
Whitespace collapse/preservation, long-word handling, explicit line breaks,
maximum lines, and error/clip/ellipsis overflow are explicit options. Width is
shaped **advance**, not glyph ink or a rotated bounding box. Bearings and
descenders remain in each returned measurement's actual ink bounds. A single
cluster wider than the width follows the configured long-word policy rather
than being split into invalid Unicode pieces.

`line_height` explicitly sets baseline spacing as a ratio of font size. Rotated
labels advance subsequent baselines along the rotated block direction. In the
FontRegistry adapter, the requested line box uses the primary selected face's
real ascent/descent and centered leading. These are font-defined metrics,
not measurements of HTML/CSS line boxes. Font family/weight selection matches
CPU rendering; undeclared Scene fonts retain legacy registration order.
An empty registry returns an error even for empty runs, without publishing a
partial Scene. No system font discovery or synthetic bolding is introduced.

Scene replacement preserves and reindexes per-label font declarations, series
ownership, clipping, and explicit drawing order. Source text is replaced by the
accepted ordinary per-line text, so applications that need the complete original
paragraph must retain it. Rewrap from that original after data, width, font or
resource revisions change; do not repeatedly wrap an already shortened result.
Measurement-call/byte budgets bound repeated shaping, while Scene and expanded
clip budgets apply before publication. Any rejected proposal leaves the Scene
unchanged. Callback-owned external side effects cannot be rolled back.

The result renders through existing SVG, Canvas, CPU and Glory label paths.
`scene_with_painted_labels` can additionally export the accepted lines as
font-independent paths; keep the text Scene for editing and accessibility.

Automatic vertical wrapping rejects explicitly. Horizontal `raster-bidi` mode
reshapes each output line independently in logical source order. Each line is
an independent bidi paragraph: `Auto` resolves its direction again, and this is
**not** full paragraph-wide bidi ordering followed by CSS line breaking.
Ordinary SVG/Canvas text still depends on the browser's installed/supplied font
and bidi behavior; path export uses the exact supplied native shaping profile.
`wrap_scene_label` now marks generated lines with the explicit
`LabelWhitespace::Preserve` protocol. Standalone/retained SVG, Canvas, CPU and
ordinary Glory Text preserve the normalized single-line spaces; the original
Label string remains available. Standalone `wrap_label` does not edit a Scene:
set its appended line indices to Preserve yourself. Each separator scalar maps
to one space (CRLF is two), while the wrapper handles hard paragraph breaks
before creating the lines. This is not CSS tab-stop layout or a change to global
host CSS. See [explicit whitespace](text-whitespace.md) for measurement identity,
backend capabilities, source migration and the tested browser bounds. Native
font advances can still differ; path export provides the fixed native shaping
profile rather than proving ordinary text pixel equivalence.

The executable `text_wrapping` demonstrates actual SVG/Canvas/CPU exports.
The gap list continues to track full CSS/paragraph layout, vertical auto columns,
and broader host/device acceptance separately.
