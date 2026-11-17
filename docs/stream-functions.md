# Stream function configuration

This development API is not included in the published 0.1.0 package.
Stream supports eight synchronous, borrowed functions through `StreamFactories`.
They participate in layout construction, including actual paint, computed
metadata, legends, tooltips, slices and [typed dot Widgets](stream-widgets.md).
The functions are never retained. [中文](stream-functions.zh-CN.md)

## Functions and contexts

Use `stream_layout_with_factories` for ordinary dot geometry, or
`stream_layout_with_factories_and_dot_factory` to combine the functions with
the existing custom dot geometry factory. Unspecified functions inherit the
existing options. Neither constructor adds fields to `StreamLayoutOptions`.

| Bundle field | Input | Output |
| --- | --- | --- |
| `label` | `StreamLayerPathContext` | Layer label `String` |
| `color` | `StreamLayerPathContext` | Layer `Color` |
| `formatted_value` | Original non-null `f64` | Formatted `String` |
| `border_color` | `StreamLayerStyleContext` | Area border `Color` |
| `dot_size` | `StreamDotStyleContext` | Circle diameter in Scene pixels |
| `dot_color` | `StreamDotStyleContext` | Dot `Color` |
| `dot_border_width` | `StreamDotStyleContext` | Border width in Scene pixels |
| `dot_border_color` | `StreamDotStyleContext` | Dot border `Color` |

All functions return `Result`. The three context types borrow the original
`BarSeries`, original series index and plot bounds. `StreamLayerPathContext`
also supplies the original layer ID and optional area `Geometry` in full Scene
coordinates. It supplies Rust geometry, rather than an SVG path string.
`StreamLayerStyleContext` supplies the complete computed layer and optional
path. `StreamDotStyleContext` supplies that complete layer, the current sample,
its configured position and the declared `StreamDotOptions` in `style`.
The layer contains every sample, including nullable samples; it is never a
partially built prefix.

## Complete application

This [example](../examples/stream_functions.rs) mounts a real Glory Memory
Chart, rebuilds its source and checks the accepted formatted value. Memory
exercises the host protocol; it does not demonstrate operating-system input.

```rust
//! A complete Glory application using Stream's eight borrowed functions.
use std::cell::Cell;

use glory::App;
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;
fn source(value: f64) -> Result<SceneInteraction> {
    let categories = [CategoryKey::from("first"), CategoryKey::from("last")];
    let series = [
        BarSeries::new("sales", vec![Some(10.), Some(value)]),
        BarSeries::new("returns", vec![Some(2.), None]),
    ];
    let options = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            ..Default::default()
        },
        dots: Some(StreamDotOptions::default()),
        legend: true,
        ..Default::default()
    };
    let formats = Cell::new(0);
    let mut label =
        |ctx: &StreamLayerPathContext<'_>| Ok(format!("{} ({})", ctx.id, ctx.series.values.len()));
    let mut color = |ctx: &StreamLayerPathContext<'_>| {
        Ok(if ctx.id == "sales" {
            Color(30, 140, 90, 255)
        } else {
            Color(190, 80, 40, 255)
        })
    };
    let mut format = |v: f64| {
        formats.set(formats.get() + 1);
        Ok(format!("{v:.0} tickets"))
    };
    let mut border = |ctx: &StreamLayerStyleContext<'_>| Ok(ctx.layer.color);
    let mut size = |ctx: &StreamDotStyleContext<'_>| Ok(8. + ctx.sample.source_index as f64 * 2.);
    let mut dot_color = |ctx: &StreamDotStyleContext<'_>| Ok(ctx.layer.color);
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut dot_border = |_: &StreamDotStyleContext<'_>| Ok(Color(255, 255, 255, 255));
    let layout = stream_layout_with_factories(
        &ChartConfig::default(),
        &categories,
        &series,
        &options,
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut dot_color),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut dot_border),
        },
    )?;
    assert_eq!(formats.get(), 3); // Three Some values; area summaries reuse these strings.
    assert_eq!(layout.layers[1].samples[1].formatted_value, None);
    assert_eq!(
        layout.layers[0].samples[1].formatted_value,
        Some(format!("{value:.0} tickets"))
    );
    let mut frame = layout.into_scene_interaction()?;
    // Bound Glory charts explicitly pick the actual painted geometry.
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    kartis::surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        kartis::surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    Ok(frame)
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Stream example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| {
        s.interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .any(|s| s.formatted_value.as_deref() == Some("30 tickets"))
    }));
    app.clear()?;
    Ok(())
}
```

## Evaluation and inheritance

The builder validates the complete source, options, stack, scales, identities,
projected sample coordinates, relevant original paths and original resource
budgets before invoking the first function. It then visits layers in existing
layout order. Within each layer it formats every `Some` sample once, evaluates
`label`, evaluates `color`, and resolves the visible area's border. It then
visits eligible samples in source order, calling dot size, color, border width
and border color in that order. The custom dot geometry factory, when supplied,
runs after the corresponding dot's function styles have resolved. Existing
Scene ordering of areas followed by dots is preserved.

Area summary data uses the final source sample. Its formatted value reuses that
sample's already computed string, without another formatter call. If the last
sample is `None`, the summary value and formatted value stay `None`; the builder
does not search backwards. Rust preserves `None`, `Some(0.0)` and original
signed-zero source values. This is an explicit nullable contract, not an
emulation of JavaScript's numeric coercion of nulls.

An explicit `label` result of `""` remains empty. It is not replaced by an ID
or the static label map. The real area may still paint, but no canonical area
interaction site or anchor is invented for that empty label. Sample labels keep
their category suffix, such as `" · first"`; computed metadata and legend labels
retain the empty layer label. Without a label function, the existing static
empty-label fallback remains unchanged.

An explicit layer `color` fully replaces static per-series color lookup. It can
therefore supply a color when a valid fixed ordinal domain excludes that series
and its unknown policy is `Missing`. The declared ordinal configuration is still
validated. `layout.colors` remains that static domain and mapping; it is not a
cache of function results. Read computed layers and current Marks for the
function's actual result. Existing explicit area paint configuration continues
to apply, so a representative color does not describe every gradient pixel.

Explicit color and border-color functions do not first resolve the static
inherited color they replace. For example, a replacement can succeed where an
unused static `Contrast` lookup on transparent paint would fail. Without a
replacement, required inherited colors retain their existing validation. An
explicit area border-color function runs for a visible area even when border
width is zero. An explicit dot border-color function likewise runs for an
eligible dot when its resulting width is zero. An absent function does not
introduce resolution of an unused default border color.

The geometry factory remains the final authority over the dot's silhouette.
Its `StreamDotContext.fill` and `stroke` contain resolved paint, and `inherit`
preserves that paint. Both geometry and style contexts keep `style` as the
declared configuration; `style.size` is not rewritten with the function result.
A custom factory that creates its own geometry must choose its own dimensions.
The size function determines the ordinary circle's diameter.

## Eligibility and limits

Label, layer color and value formatting run for hidden layers too. Area border
functions run only when a visible area exists. Dot functions and the custom dot
geometry factory require enabled dots, a visible layer and a non-null sample.
They still run for `Some(0.0)` and zero-size dots. `dots: None` and `None` samples
retain existing slice placeholders without becoming dot function roles.
Host-level hiding is a separate presentation operation.

A hidden 1,024-byte layer label is valid when no hidden sample Mark is emitted;
the builder does not concatenate a nonexistent Mark's category suffix. Hidden
area paths are also omitted when no label or color function requests a path.
Supplying either of those functions requests the original geometry, which must
pass the same finite-coordinate checks, including the absolute 1,000,000-pixel
control-coordinate bound. This matters for Natural curves whose controls can
extend beyond their sample coordinates. A formatter or dot function alone does
not request that unused hidden area path.

Function strings are bounded to 1,024 UTF-8 bytes. Visible generated sample
labels must fit that same budget after concatenation. Dot diameter must be
finite and between 0 and 4,096 pixels. Border width must be exactly zero or
between 0.000001 and 256 pixels. Source, Scene command, paint-stop and identity
budgets still apply; function outputs and custom geometry are checked as they
are produced. A later error returns no partial layout, but cannot undo external
side effects performed by an earlier callback.

Combining `dot_size` with any configured glyph is rejected before callbacks:
circle diameter and glyph area or nominal size use different units. The other
functions can be combined with glyphs and custom geometry. A stroke-only glyph
may obtain its required border from `dot_border_width`; a returned zero width
is rejected before its geometry callback. With no eligible dot, the builder
does not invent a stroke-only role merely to evaluate a function. The old
static validation remains unchanged when no width function is supplied.

## Scope of compatibility and evidence

Nivo Stream exposes function-valued label, colors, value formatting, area
border color and four dot style properties. Its layer label/color accessors
receive an ID and generated path; dot styles receive computed data. Kartis uses
the borrowed Rust contexts above. Its flat series input, typed category keys,
nullable values and `Geometry` paths are explicit Rust interfaces, not general
JavaScript property getters or numeric-ID/string-ID equivalence.

The Rust builder formats hidden samples and resolves hidden labels/colors;
border and dot calls follow actual Rust area/dot eligibility. Nivo's rendering
calls also depend on mounted layers and `enableDots`, while its public Stream
datum value is numeric. Those qualification differences are not hidden behind
a claim of identical callback traces. Interaction enablement does not eliminate
the creation of an otherwise eligible zero-size dot role.

The recorded 96-profile comparison uses the actual pre-change compiled builder
and compares default Debug layouts and SVG bytes with the refactored default
path. It covers those recorded configurations, not every possible input.
Additional checks exercise full-layer contexts, nullable formatting, explicit
color overrides, hidden admission boundaries and typed Memory/CPU host
transactions. Browser evidence must distinguish declared metadata, actual source
paint, legend DOM labels and input; no source glyph-label or Widget font fixture
is implied by this function example.

These functions do not implement React components, automatic springs, arbitrary
JavaScript layer execution, or universal cross-renderer text and pixel identity.
Live Widgets remain a separate [dot-role adapter](stream-widgets.md); scene
exports contain the resulting Scene paint, not live Glory children. GPU and
worker rendering are separate capabilities.
