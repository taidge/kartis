# Live Bar components (Unreleased)

`bar_widgets` builds typed measured component packets from `CustomBarLayout`.
Use `bar_layout_with_factories(..., BarFactories::default())` for inherited
ordinary Bar paint. The same entry supports grouped, stacked, percent,
horizontal and vertical layouts, category windows, labels and totals.
Geometry/label factories and the configured-scale variants use the same packet
API. Existing public `StyledBarLayout` structure remains unchanged.

```rust
use kartis::*;
let categories = vec!["A".into(), "B".into()];
let series = [BarSeries::new("observed", vec![Some(3.), Some(0.)])];
let layout = bar_layout_with_factories(
    &ChartConfig::default(), &categories, &series,
    &BarOptions::default(), &BarStyleOptions::default(), BarFactories::default(),
)?;
let widgets = bar_widgets(&layout, 0, |_| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-12., -5.],
        measured: TextSize { width: 24., height: 10. },
    }))
})?;
let frame = bar_widget_frame(&layout)?;
# Ok::<(), kartis::ChartError>(())
```

Each `BarWidgetData` owns a shared `Arc<BarSeries>`, a shared complete category
vector, the current `StyledBarDatum`, the logical position and plot, original
unclipped rectangle, unclamped endpoint projections and orientation. Full raw
series include None, hidden and cropped values. Raw values preserve their
original numeric sign bits. `bar.displayed_value` and `bar.interval` retain
stack/percent semantics separately from the raw source. `bar.color` is the
original numeric palette color; current custom paint belongs to the host frame's
actual Mark. One original series clone is shared by its selected bars, and one
category vector is shared by all roles and layout clones.

`label` and `label_clip` contain the current Scene-owned label and clip. They
preserve label-factory text, including an empty string, and reflect accepted
`set_active` label-color changes. `None` indicates that the Rust label layout did
not emit that label. Labels and totals keep their Scene ownership when mark ink
is replaced; reading label metadata does not automatically render a second copy.
The placement context offers borrowed versions of these values, the original
series/category, current bar and actual Mark.

`original_rect` may be `None` when the unclipped rectangle cannot be represented
within its finite coordinate profile. Each original endpoint may independently
be `None`. These fields do not invent a crop-edge rectangle or zero value.
`bar.visible_rect` describes ordinary cropped geometry; a custom Path can paint
away from it. `AtDatum` resolves the accepted original logical anchor rather
than the custom ink's bounding-box center.

Private source snapshots are captured during the factory layout, before callers
can edit its public frame. The complete accepted Scene/metadata, original raw
indices, values, categories, positions, actual marks and typed binding are
checked before any placement callback. Editing a managed public frame requires
rebuilding it through the layout API. `set_active` remains the supported
controlled-opacity update and retains original data and placement. Scene numeric
scalars remain compatible paint values, distinct from the owned original datum.

Only actual emitted bars invoke placement. None sources, hidden series and
fully cropped bars have no role. Existing Some(0), zero-area and opacity-zero
bars still have roles; host clipping and visibility govern painting and input.
An empty layout retains its exact plot through `bar_widget_frame`.
Returning `None` skips a selected role. Callbacks are borrowed, never retained,
and at most 128 selected slots are allowed. Errors return no partial packet;
external callback side effects are not rolled back.

Use `plot_widgets_with_data` for an overlay or `PlotWidgetsComposition` with
`plot_widgets_composed` for transactional source-ink replacement. Browser
source-order composition uses `mount_with_plot_widgets_in_scene` and its matching
setter. `bar_widget_frame` keeps Scene, source binding, Path anchors and plot
together with the explicit solid-paint `PaintedGeometry` profile. CPU image
Charts use `chart_layout(frame).raster(fonts, options)`, with slot projection
following the accepted image camera independently of image DPR. Hosts perform
final paint/clip admission. Failed packets, child construction or PNG publication
retain accepted source, controls, component state and focus. Explicit revisions
request new child scopes.

Nivo filters null values before creating `barComponent`, while retaining zero.
Its bars layer gate determines component creation; interactivity and label
visibility are separate props. The original transition recipes collapse height
for vertical bars and width for horizontal bars. The frozen verifier executes
those original recipes, common props, component arguments, label layout and
default rectangle clamps. It does not run React reconciliation or spring timing.
Rust source-series/category modeling, clipping eligibility and zero-label
policies remain explicit Rust contracts rather than prop-by-prop equivalence.

`cargo run --example bar_widgets` runs a complete Memory application. The browser
acceptance driver is `scripts/bar-widgets-acceptance.mjs`; CI uses
`--chromium-only`. `node scripts/bar-widget-reference.mjs --frozen` verifies the
original sources without a checkout or network. Scene/SVG/PNG exports retain
authored source paint and omit arbitrary live Widget content. Automatic springs,
React animated props, arbitrary component exports, native same-layer input and
GPU/worker remain open. Published 0.1.0 does not include these APIs.

Browser pixel samples use explicitly solid-color Buttons with font-size zero;
their JSON text still supplies checked data/state. Separate label stages check
owned text, clip and dimmed metadata with actual Widget input suppression.
An earlier font8 same-color glyph sample has a channel error of two in the actual
Canvas compositor and remains an unaccepted text diagnostic. A reversible
font8/0/8 control preserves bounds and state; an independent blank native Button
did not reproduce it. The solid-color matrix does not establish text raster
equivalence, and its color tolerance remains ±1.
