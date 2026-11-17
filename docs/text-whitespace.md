# Explicit label whitespace

[简体中文](text-whitespace.zh-CN.md)

This API is available in Kartis 0.2.0.
It defines a single-line drawing protocol, not a CSS whitespace or paragraph
layout engine.

`LabelWhitespace::Legacy` is the default. An absent entry keeps each backend's
existing behavior; ordinary scenes do not acquire new whitespace styling.
`LabelWhitespace::Preserve` retains leading, trailing and consecutive spaces.
Before measurement and drawing, each tab, CR, LF, VT, FF, NEL, line separator
and paragraph separator becomes one U+0020. CRLF therefore becomes **two spaces**
in a single Label. NBSP, narrow NBSP, combining characters and joiners are not
rewritten. The Scene's authored text remains unchanged.

Select a mode by label index:

```rust
# use kartis::*;
# fn main() -> Result<()> {
let mut scene = ChartConfig::default().scene()?;
scene.labels.push(Label {
    x: 30., y: 40., text: "  Revenue\tforecast  ".into(),
    size: 18., color: Color(20, 40, 75, 255),
    anchor: TextAnchor::Start, rotation: 0.,
});
scene.typography.label_whitespace.insert(0, LabelWhitespace::Preserve);
scene.validate()?;
# Ok(())
# }
```

`SceneTypography::label_whitespace(index)` resolves missing entries to Legacy.
`LabelWhitespace::normalize` and `drawable_label` expose the same normalization
without mutating the source. Sparse entries must reference existing labels and
follow the ordinary Scene label budget. `remap_labels` remaps both font and
whitespace entries, dropping removed slots.

Standalone and retained SVG apply preserving text styling only for Preserve;
switching back to Legacy removes it. Canvas and CPU receive the same normalized
single-line string. Ordinary Glory Text keeps a real Text node and uses the
explicit preserving style for this mode. Mode changes participate in retained
caches, so changing only the mode updates the accepted frame. A custom Canvas
backend must opt into `supports_text_whitespace`; otherwise a Preserve Scene
rejects before `begin`, rather than silently collapsing spaces.

## Wrapping and measurement

`wrap_scene_label`, including the FontRegistry adapter, automatically marks
every generated line Preserve and reindexes the other modes transactionally.
The wrapper processes paragraph separators as hard breaks **before** it emits
single-line labels; this differs from directly drawing a Label containing CRLF.
The standalone `wrap_label` returns line data without changing a Scene. When
appending those labels manually, also set their Scene whitespace entries to
Preserve. See [measured wrapping](text-wrapping.md).

`SemanticTextRequest.whitespace` is part of measurement identity. Existing
`for_label` constructs a Legacy request; `for_label_with_whitespace` constructs
an explicit one. A measurement from the same raw string and font in another
mode cannot satisfy the new request. Resource revision still needs updating
when fonts change. Real host line boxes and native font advances remain
host-dependent.

AreaBump, hierarchy and Sunburst provide `with_label_whitespace` before measured
placement. Their candidates expose the selected mode for the real measurer;
signatures bind it, and hidden/restored labels keep their mode. Directly editing
a managed label's Scene entry does not refresh its old measurements and rejects
placement. Use the layout's mode-selection method and obtain new measurements.

## Compatibility and verified scope

New public fields include `SceneTypography.label_whitespace`,
`SemanticTextRequest.whitespace`, and measured-candidate whitespace fields.
These additions can break full struct literals. Use `Default` plus mutation
for SceneTypography and request/layout constructors where available; otherwise
supply the new field. This is not a claim of complete 0.1.0 source compatibility.

Four real browser engines at DPR 1/2 passed 8 contexts and 32 phases involving
SVG, Canvas and ordinary Glory Text, including mode-only changes. The declared
gates are a 1 logical-pixel width bound and a 1.5 logical-pixel SVG-image/Canvas
ink-bounding-box bound. They do **not** establish whole-frame RGBA equality or
repair the previously observed Firefox/CPU advance difference. Font selection,
kerning, bidi and full CSS layout remain separate concerns; supplied-font path
export remains available for a fixed native shaping profile.
