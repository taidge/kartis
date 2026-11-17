# Typed HeatMap and Waffle cell Widgets

[简体中文](cell-widgets.zh-CN.md)

Development APIs, not included in the published 0.1.0 package. These adapters
supply real Glory components for the cell roles represented by Nivo's HeatMap
and Waffle `cellComponent`. They do not retain React callbacks, implement
`animatedProps`, or create automatic spring trajectories.

## Source and measured placement

`heatmap_cell_widgets(&layout, revision, placement)` returns
`PlotWidgetsData<HeatmapCellWidgetData>`. The borrowed placement context exposes
`layout`, original `datum: &HeatCell`, original `source_index`, current computed
`cell` and actual `mark`. Owned data retains that original datum and index,
current computed cell and plot. Configured row/column order does not rewrite
input indices. `None` and `Some(0)` remain distinct; sparse absent pairs are not
synthesized. Hidden cells have no marks and do not invoke placement. Existing
visible zero-sized or zero-opacity marks still invoke it.

`waffle_cell_widgets(&layout, revision, placement)` returns
`PlotWidgetsData<WaffleCellWidgetData>`. Context exposes `layout`, full-slot
`cell`, optional owning `group` and actual `mark`. Owned data contains the cell,
`Option<Arc<WaffleGroup>>`, total, unit, grid and cell size. An empty cell has no
source group, rather than a synthetic group with value zero. Original numeric
values remain numeric independently of formatted captions. Selected cells in
one group share one owned snapshot; metadata equality compares values.

Only actual cell marks invoke Waffle placement. In `WaffleMode::Areas`, occupied
cells belong to solid group unions and have no individual marks; they are
skipped. Empty cell marks still participate. This entry does not provide an
area component or change Kartis Areas into Nivo's transparent interaction layer.

Both adapters validate the complete input before invoking any placement
callback. Returning `None` skips a cell. At most 128 selected slots are admitted;
large source layouts can select a subset. The packet includes the layout's
actual plot. Receiving hosts validate measured dimensions, clipping and their
supported capabilities. No callback is retained. Errors return no partial
packet; external callback side effects cannot be undone.

## Current paint and original business data

Use `context.mark` or the live `frame.frame.mark` for actual geometry, fill,
stroke and opacity. A custom cell can be a checked Path or Circle rather than
the original rectangle. Cell bands/full-slot bounds and logical origins do not
follow offset custom ink. Choose an explicit rectangle or a measured `AtDatum`
placement; Path datum placement requires its accepted typed anchor binding.

HeatMap authenticates original nullable source with its private provenance and
retains original input order privately. Default layouts keep their existing
public appearance-editing policy: cell geometry must agree with actual Mark
geometry, and opacity must agree with the accepted profile. Palette metadata
is distinct from actual decorated fill. Row/column order arrays may be edited
but cannot contain duplicate or unknown identities. Custom-factory layouts
keep their stricter existing geometry, paint and owned-label guards. Waffle
checks current public allocation, grid, groups, actual marks and typed sites.
Neither adapter infers raw data by parsing display text or Mark scalar defaults.

Explicit typed packets can attach to real decorative marks with an empty label,
which is necessary for Waffle empty cells. The original Mark is unchanged.
This does not create a source logical datum, canonical control or picking site.
The untyped slot API keeps its previous data-Mark eligibility rule. A child
Widget owns its own pointer and keyboard actions; activation does not implicitly
select the underlying source.

## Choose a composition host

Pass the packet to `Chart::plot_widgets_with_data` or browser typed overlay
methods when original source ink should remain. For replacement, wrap it in
`PlotWidgetsComposition { widgets, replace_ink }`; replacement IDs must be real
selected slot IDs. Use the [strict composition](widget-composition.md) API, or
[Scene-order browser composition](ordered-widgets.md) for interleaved SVG/Canvas
and component content. All original Scene labels remain in their authored
positions; a cell replacement does not infer ownership of those labels.

ID and revision preserve the child scope and local state while accepted data
changes. Waffle IDs identify physical grid slots: a new allocation can change
the owner of a retained cell component. Change revision when application state
should reset. A layout rebuild that removes a cell's actual mark and slot
disposes that component. Host legend filtering or zero opacity can instead
retain a still-present slot while hiding it and disabling input.
Positive source opacity/color do not automatically style arbitrary Widget
content. Read the accepted frame and choose styles explicitly.

For native semantic `chart_layout`, HeatMap offers
`into_painted_scene_interaction()`: it preflights the supported solid painted
geometry and retains nullable sites and custom anchors. The ordinary
`into_scene_interaction()` preserves legacy geometry picking for browser and
Surface hosts. Semantic Graphic/text capability limits still apply.
For the first browser mount, pass the frame's Scene and place its binding in
`BrowserOptions::interaction_data`; set `options.interaction.plot` to the same
frame plot. This admits nullable controls and custom anchors before the first
component factory runs. Use the layout-and-packet setter for later updates.

The existing `raster_chart` accepts Scene and measured typed Widgets, not a
`SceneInteraction` binding. Its PNG publication gate also gates strict child
updates. Child metadata retains nullability. Scene-only source controls and
tooltips cannot infer nullable values from the compatibility scalar zero;
disable those controls for nullable HeatMap image examples and use actual child
controls. `RasterSurface` separately accepts typed bindings but has no live
Widget composition entry. For an explicit PaintedGeometry binding, the new
[`chart_layout(frame).raster(...)`](raster-chart-bindings.md) path retains typed
source input alongside live Widgets instead of inferring data from Scene scalars.

## Complete examples

[HeatMap components](../examples/heatmap_widgets.rs) and
[Waffle components](../examples/waffle_widgets.rs) mount actual controls in a
Glory Memory App. They compile without optional features:

```sh
cargo run --example heatmap_widgets
cargo run --example waffle_widgets
```

For an application, select its Glory host and supply a component factory reading
`context.frame`, a Bond of accepted placement and metadata. Keep proposals under
one application-owned source when Scene, binding and packet must change
together. Strict failure preserves the previous accepted source and children;
the application's rejected proposal is not rewritten.

Scene/SVG/PNG exports omit arbitrary live Widget content. Exporting the original
Scene still draws its authored ink. Browser input uses the existing bounded
Canvas-alpha hit profile, also for SVG. Native same-layer input, arbitrary
component export, automatic springs and GPU/worker remain open. See the
[feature gaps](feature-gaps.md) and [verification](verification.md).

The independent [Waffle group event binding](waffle-regions.md) adds complete-slot business picking without changing this cell Widget adapter or the solid Areas paint mode.
