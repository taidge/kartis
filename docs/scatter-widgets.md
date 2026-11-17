# Live Scatter node Widgets (Kartis 0.2.0)

`scatter_node_widgets(&layout, &series, revision, placement)` builds measured,
owned component packets for actual Scatter node marks. Pass the original
`ScatterSeries` input explicitly: the existing public `ScatterLayout` does not
retain every raw datum field. This preserves its public structure and existing
editing contract while giving children complete original data.

```rust
use kartis::*;
let series = [ScatterSeries::new("measurements", vec![
    ScatterDatum::new("first", 1., 4.),
    ScatterDatum::new("zero", 2., 0.),
])];
let layout = scatter_layout(&ChartConfig::default(), &series, &Default::default())?;
let widgets = scatter_node_widgets(&layout, &series, 0, |_| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16., -11.],
        measured: TextSize { width: 32., height: 22. },
    }))
})?;
# Ok::<(), kartis::ChartError>(())
```

Each `ScatterNodeWidgetData` owns a shared `Arc<ScatterSeries>`, the current
`ScatterSite`, and the plot. The original datum is
`data.series.data[data.site.source_index]`. Selected nodes in one series share
one clone of that complete series. Unselected series are not cloned. Raw XY
retain Number/String identity and original numeric sign bits; optional size,
original labels and explicit source colors remain available. Computed indices,
formatted XY, resolved size/unit, mapped position and bounds come from the site.
Current geometry, fill, stroke and opacity come from the accepted host frame's
Mark. A source color and a custom node's representative paint can differ.

The borrowed placement context exposes the layout, series, original datum,
computed site and actual Mark. Full validation precedes any placement callback:
every raw source record, metadata/global index, resolved axis configuration,
projection, site, mark, bounds and accepted interaction anchor must agree.
Validation includes unselected and late source records. Missing-size skips,
hidden series and explicit Point-domain omissions have metadata but no actual
node role. Existing zero-size, zero-opacity and off-plot nodes still have roles.
Host clipping and visibility determine whether the measured child can paint or
receive input; an off-plot child can remain mounted and inert.

Public layout metadata can still be coherently edited. This is a consistency
check against the explicitly supplied current source, not a private immutable
snapshot. The layout does not retain the original size accessor or numeric
format options; the adapter validates resolved sizes and assembled labels.
It cannot prove those unretained options from arbitrary public edits. Site color
is current metadata and does not overwrite the Mark's actual paint. The Scene's
compatible numeric scalar does not replace typed raw XY and may represent a
string Y with zero; accepted typed data preserves the original string.

`None` from placement skips a node, and at most 128 selected slots are allowed.
The callback is borrowed and never retained. Errors return no partial packet;
external callback side effects are not rolled back. Explicit revision changes
request new child scopes. Ordinary metadata updates retain child identity,
state and focus under the existing host policy.

Use `Chart::plot_widgets_with_data` for an overlay, or
`PlotWidgetsComposition` with `plot_widgets_composed` to replace selected source
ink transactionally. CPU image Charts accept
`chart_layout(frame).raster(fonts, options)`; their accepted camera projects slots
independently of image DPR. Semantic `chart_layout` requires an explicit
`PaintedGeometry` binding. For Scatter, create `SceneInteraction` from the layout
and set its binding profile before mounting. Browser source-order composition
uses `mount_with_plot_widgets_in_scene` and its matching setter. Failed typed
data, child construction or PNG publication retains the preceding accepted
source, controls and children. Diagnostic factory counters remain side effects.

Nivo's `Nodes` creates a `nodeComponent` for each transition item, including size
zero. Its `isInteractive` flag removes handlers rather than component roles;
regular styles use node XY/size/color and leaving styles set size to zero.
The frozen original-source verifier executes those argument recipes, the real
element-creation callback, `computePoints`, and the default radius calculation.
It does not run React reconciliation, SpringValues or transition timing. Nivo's
`serieIndex` means the datum's index within its series; Kartis names these
`source_index` and `series_index` separately. Rust hidden/missing-size/Point-domain
omissions are explicit Rust contracts, not claimed Nivo filtering equivalents.

Run `cargo run --example scatter_widgets` for a complete Memory application.
The browser driver is `scripts/scatter-widgets-acceptance.mjs`; CI uses
`--chromium-only`. Original source verification uses
`node scripts/scatter-node-widget-reference.mjs --frozen` without a checkout or
network access. SVG/PNG Scene exports preserve source ink and omit arbitrary
live Widget content. Automatic springs, React animated props, arbitrary Widget
exports, native same-layer input and GPU/worker remain open. Published 0.1.0
does not include this API.
