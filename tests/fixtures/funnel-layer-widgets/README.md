# Funnel custom layers and style callbacks

This fixture preserves complete Nivo 0.99.0 Funnel, colors and core source
files, their package manifests and three complete MIT notices. The offline
verifier checks every source, license, producer and pinned compiler hash.

Original Funnel source has no `partComponent`, `labelComponent` or
`separatorComponent` property. Its function-valued custom layers receive
whole parts with handlers, area/border generators, before/after separators,
width/height and `setCurrentPartId`, in the declared layer order. Eight
parent-layer profiles execute actual `createElement` argument construction.

Sixteen profiles execute the original computed-part callback and profile
construction. Original color/inherited-color/formatter function branches
return the supplied functions. The observed per-datum order is size, raw color,
value formatter, computed-part border color, then label color. Profile arrays
are still empty at style callback time. Zero-width, zero-value and opacity-zero
parts remain eligible. `part.data` retains the original raw object and extra
generic fields. Default labels render `formattedValue`, not `data.label`.
Four handler profiles execute original current/tooltip/consumer ordering;
two renderer profiles execute default Part, label and separator JSX.

Band sizing, supplied size functions, path generators, JSX, hook scheduling
and Spring are explicitly authored dependency probes. This does not execute
D3 layout/path generators, React lifecycle, Spring timing, native events,
actual tooltips, arbitrary Glory Widgets or framebuffer equivalence.

The original separator function is also executed. Its horizontal branch
ignores the before/after flags, its terminal separator uses the string `none`,
and empty parts throw even with both flags off. These are preserved upstream
facts, not changes to make in Rust. Original Parts/PartLabels map every part;
Kartis body/border/separator Widget roles require actual source Marks. A
terminal Rust separator has no invented source datum. Typed measured Widgets
are a custom-layer adaptation and Glory extension, not an original per-part
component property or equivalent arbitrary layer-position API.

After installing the pinned project packages, verification needs no checkout,
Git, network or filesystem writes:

```sh
node scripts/funnel-layer-widget-reference.mjs --frozen
```

An optional fixture directory follows `--frozen`. Regeneration is separate:

```sh
node scripts/funnel-layer-widget-reference.mjs <nivo-checkout> [output-directory]
```
