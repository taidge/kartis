# Kartis

[简体中文](README.zh-CN.md)

Kartis is a Rust chart library for Glory. Layouts produce a shared `Scene` for standalone SVG, browser Canvas, Glory widgets, and optional CPU raster output. Layout and interaction run in Rust without a JavaScript charting library. The API draws on D3 and Nivo. [Architecture](docs/architecture.md) · [Third-party notices](docs/third-party-notices.md)

## Getting started

Kartis requires Rust 1.95 or newer. Build a chart from data and pass its scene to `chart`. The example uses Glory’s in-memory host. Applications can select a Glory host appropriate to their platform.

Add the crates.io dependencies to your `Cargo.toml`:

```toml
[dependencies]
kartis = "0.1.0"
glory = { version = "0.8.0", default-features = false }
```

Enable `web` for browser widgets and Canvas, `raster` for CPU rendering,
`raster-bidi` for bidirectional raster text, `geojson` for GeoJSON, or
`timezone` for time-zone support. [Release process](docs/releasing.md)

```rust
use kartis::{ChartConfig, LineOptions, Point, Series, chart, line};

# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let scene = line(
    &ChartConfig::default(),
    &[Series::new("Revenue", vec![Point::new(0., 20.), Point::new(1., 35.)])],
    &LineOptions::default(),
)?;
let app = glory::App::memory();
app.mount(chart(scene).on_select(|datum| println!("{}: {}", datum.label, datum.value)))?;
# Ok(())
# }
```

Pass a `Cage<Scene>`, `Bond<Scene>`, or `Vary<Scene>` to update a widget reactively. Kartis validates each proposed scene before publishing it and keeps the previous frame if validation fails. Stable mark IDs preserve widget identity across updates.

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let app = glory::App::memory();
let scene = app.scope().cage(ChartConfig::default().scene()?);
app.mount(chart(scene).on_reject(|error| eprintln!("{error}")))?;
scene.revise(|frame| frame.title = "Updated chart".into());
app.flush()?;
# Ok(())
# }
```

## Chart families

| Family | Layouts and capabilities |
| --- | --- |
| Cartesian | Line and area, dual axes, grouped and stacked bars, scatter, HeatMap, stream, bump, AreaBump, and Marimekko |
| Radial | Pie and donut, radar, polar axes and bars, stacked radial bars, and chord |
| Statistical | Box plots from samples, bullet charts, distributions, swarm, planar contours, and weighted density |
| Hierarchy and flow | Tree, treemap, circle packing, sunburst, icicle, funnel, Sankey, and force-directed networks |
| Composition | Waffle grids with rounded cells or contiguous areas |
| Spatial and time | GeoMap, choropleth, geographic routes and symbols, Delaunay, Voronoi, calendar, and compact time ranges |

### Scatter, scales, and symbols

`scatter_layout` accepts stable node IDs and numeric, UTC, or point scales on both axes. It returns the scene together with raw values, formatted values, source indices, resolved scales, and interaction anchors. Its default marker diameter is 9 px. The original `scatter` API uses a default radius of 5 px.

`ScatterNodeShape` distinguishes circle diameter, filled glyph area, and the D3 nominal size for stroked glyphs. Sizes can be fixed or mapped from a datum’s size value, with explicit clamp and missing-value policies. Exact picking uses the painted shape and clip. Nearest picking uses each node’s logical origin, including zero-size nodes. The layout can supply a typed interaction binding and independent X/Y viewport domains.

`scatter_layout_with_node_factory` lets you draw a custom node from its raw data, mapped position, resolved size, and inherited style. Return a `ScatterNodeGeometry` to provide a path or a centered primitive with custom fill, gradient, stroke, or dashes. Offset paths retain the data position as their interaction origin.

`ContinuousScaleOptions` supports reverse, nice, round, clamp, and logarithmic base. `line_layout_with_scale_options` shares the resolved scale across axes, paths, labels, and interaction. `bar_layout_with_scale_options` uses a shared value scale for axes, grouped and stacked bars, and totals, including positive powers and square roots. `PointScale` and `point_line_layout` use a zero-bandwidth category scale. A numeric key and a string key keep distinct identities even when they display the same text.

Use `ScaleKind::Pow { exponent: 2. }` or `ScaleKind::Sqrt` for signed nonlinear numeric coordinates. Ticks and nice remain in source-value units. Shared chart layouts and domain viewports use the same transform; `scales::PowScale` also exposes standalone mapping and inversion. Negative exponents require one same-sign nonzero branch; zero exponents and overflowing or collapsed transforms reject. Zero-baseline bars and areas require positive powers. A reciprocal viewport cannot cross its original zero singularity.

`glyph_geometry` provides seven filled and seven stroked symbols. Filled size denotes area. Stroked size follows the D3 symbol convention. The factory returns geometry, bounds, and the original anchor. Line, Radar, Scatter, Swarm, Network, and Geo layouts share these symbols. Geographic glyphs can use projection masks and matching size and color legends.

`swarm_layout` fixes each node at its mapped value and resolves collisions along the other axis using an envelope around the symbol and its stroke. It supports numeric and UTC scales, explicit group order, per-node sizes, and typed interaction anchors.

`contour_layout` retains planar islands and holes in the shared SVG, Canvas and CPU Scene. `estimate_density` accepts signed weights; `scatter_density_layout` preserves original Scatter identities and places density behind its source marks. `recontour` reuses the accepted field for thresholds and paint; changed source inputs or estimator settings require rebuilding. [Contour and density guide](docs/contour-density.md)

Use `combo_layout_with_scale_options` for mixed charts with independently configured value axes. Its layout exposes the resolved scales, category window or continuous X mapping, and interaction sites for the visible marks.

`radar_layout_with_value_scale` supports linear, logarithmic, and symmetric-log radial scales. It returns the resolved scale and grid ticks alongside the layout. Choose value-scale ticks or evenly spaced rings with values from the inverse scale.

`radar_layout_with_point_factory` lets you draw a Radar point from its raw value, mapped position, and inherited paint. An offset path keeps its mapped data position for nearest-point interaction. `bullet_layout_with_scale_options` uses the resolved value scale for the tracks, markers, and axis ticks.

`line_layout_with_point_factory` and `point_line_layout_with_point_factory` accept custom point shapes. The factory receives the original numeric or category value. `boxplot_layout_with_scale_options` shares a numeric or UTC value scale between the statistical summary, axis, and drawn marks.

`AreaBumpLayout::place_endpoint_labels` places measured start and end labels along the sides of the plot. Pass actual text bounds from the application to keep labels clear of each other and configured obstacles.

`layout_stacked_radial_bar_with_arc_style` applies a configurable corner radius and D3 arc padding to both segments and tracks. Each segment keeps its raw cumulative angles and stable mark ID for interaction.

Pie and RadialBar labels accept `ArcLabelRadiusMeasure::RingThickness` to match Nivo's thickness-based `skipRadius` filter. `OuterRadius` preserves their original behavior. Both typed layouts expose validated interaction bindings with original values and polar anchors, including active slice expansion and rounded arcs.

`funnel_layout` exposes each stage’s source data, projected profile, separators, and interaction target. Configure the direction, linear or smooth sides, stage spacing, and controlled highlighting.

`waffle_layout` exposes the grid slots and each group’s allocation. It supports four fill directions, independently rounded counts or largest-remainder allocation, and either padded cells or solid group areas.

`sunburst_layout` uses a D3 equal-area partition with optional rounded and padded arcs. Nodes retain both their own value and their subtree total. Children can inherit parent colors, and each visible arc has a stable interaction target.

Sunburst also offers measured `label_candidates` and `place_labels`. Supply rotated ink bounds relative to each label anchor, then choose plot placement, clipping to the actual rounded arc, or conservative full-bounds admission. Priorities, offsets, controlled hiding, restoration, and mask work limits apply without changing the default layout.

`chord_layout` keeps both matrix directions in each ribbon and chooses the larger direction as its source. The rim and ribbon radii are configured independently. `ChordLayout::tooltip_text` formats the two endpoint rows from the original values.

`bump_layout` maps typed categories and distinct ranks to independent point scales. It retains the transition shoulders and missing slots. `bump_layout_with_point_factory` draws custom points while keeping their original interaction anchors.

`stream_layout` exposes the source values, stack boundaries, projected samples, and layers. Configure stack order, offset, curve, axes, and dots. Use `into_scene_interaction` for layer picking or `into_slice_interaction` for sample slices, including samples whose dots are hidden.

`StreamDotOptions::glyph` selects a shared filled or stroked symbol. `stream_layout_with_dot_factory` accepts custom point geometry and paint. Start, center, and end positions keep their stack anchors, including when the painted point is offset. Exact picking follows the point's ink and clip; nearest picking and slices retain the original data position.

### Full-breakpoint scales and geographic inverse

`PiecewiseScaleOptions` retains full numeric or UTC domain and range stops. Use `line_layout_with_piecewise_scale_options` or `ScatterAxisScale::Piecewise` for actual chart axes, geometry and typed picking. Their range stops are plot fractions starting at 0 and ending at 1. Domain viewports rescale every knot and pass `piecewise_domains` to the layout callback. Turning ranges support forward drawing and an explicit `invert_segment`; a global inverse and data viewport require a strictly monotonic range. [Scale and inverse guide](docs/advanced-scales.md)

```rust
# use kartis::*;
let profile = PiecewiseScaleOptions {
    domain: vec![0., 2., 10.], range: vec![0., 0.75, 1.],
    ..Default::default()
};
let layout = line_layout_with_piecewise_scale_options(
    &ChartConfig::default(),
    &[Series::new("Observed", vec![Point::new(1., 2.), Point::new(6., 8.)])],
    &LineOptions::default(), &LineStyleOptions::default(),
    &PiecewiseLineScaleOptions { x: Some(profile), ..Default::default() },
)?;
let controller = layout.viewport_controller(ViewportLimits::default())?;
assert_eq!(controller.piecewise_domains()?.x, Some(vec![0., 2., 10.]));
# Ok::<(), ChartError>(())
```

`GeoProjectionContext::invert` returns longitude/latitude using the accepted fit, camera, scale and translation. `invert_in_plot` additionally checks the plot rectangle. Browser and raster handle helpers convert client or host coordinates through the accepted Scene camera once; keep the projection context with the frame it describes.

```rust
# use kartis::*;
let map = geographic_map_layout(
    &ChartConfig::default(), &[],
    &GeoMapOptions { projection: GeoProjection::Orthographic, ..Default::default() },
)?;
let pixel = map.projection.project([10., 20.])?.unwrap();
let geographic = map.projection.invert(pixel)?.unwrap();
assert!((geographic[0] - 10.).abs() < 1e-8);
assert!((geographic[1] - 20.).abs() < 1e-8);
# Ok::<(), ChartError>(())
```

`GeoProjection` also supports checked Conic Equal Area, Equidistant, Conformal and the fixed Albers preset. Maps, routes and symbols share the fitted projection context and its finite footprint. Data fit uses conservative curved-edge bounds; Conformal has a finite latitude cap. [Conic and Albers guide](docs/geo-conic.md)

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let projection = GeoProjection::conic_equal_area([29.5, 45.5])?;
let map = geographic_map_layout(
    &ChartConfig::default(), &[],
    &GeoMapOptions { projection, ..Default::default() },
)?;
let pixel = map.projection.project([-95., 35.])?.unwrap();
let geographic = map.projection.invert(pixel)?.unwrap();
assert!((geographic[0] + 95.).abs() < 1e-8);
assert!((geographic[1] - 35.).abs() < 1e-8);
# Ok(())
# }
```

### Paint and formatting

`Scene` supports ordered layers, labels owned by a data series, plot and path clips, gradients, patterns, and dashed strokes. SVG and Canvas use shared paint semantics, including a resolved footprint for a Round or Square stroke on a single point.

`MarkBlends` assigns artistic blend modes by mark ID. SVG, Canvas, and CPU raster can blend a mark's fill and stroke together against the chart's background and earlier draws. Use the `*_with_blends` render functions or `BrowserChart::mount_with_blends` for a retained chart. Pass the target blend profile to `animate_to_with_blends` to animate the retained chart.

`ColorScaleOptions::resolve_with_profile` accepts an authored palette with a color interpolation space or one of 38 named Nivo/D3 ramps. HeatMap, calendar, and choropleth layouts accept the same profile. Pass the resolved scale to `append_color_legend` to use its domain, missing-value color, and lookup-table boundaries.

`ColorScaleTransform` selects Linear, Log, Pow, Sqrt or SymLog inputs for continuous and diverging colors. The chart and legend share the transform, including the actual position of an asymmetric diverging center. Log domains use one nonzero sign; reciprocal powers retain one nonzero branch. Nonrepresentable inverse queries return an error.

```rust
# use kartis::*;
let scale = ColorScaleOptions {
    transform: ColorScaleTransform::Log, domain: Some([1., 1_000.]),
    ..Default::default()
}.resolve([Some(1.), Some(10.), Some(1_000.)])?;
assert!((scale.position(10.).unwrap() - 1. / 3.).abs() < 1e-12);
assert!((scale.value_at(2. / 3.)? - 100.).abs() < 1e-10);
# Ok::<(), ChartError>(())
```

For discrete colors, choose `ColorScaleKind::Quantile` to derive R7 cuts from the non-missing observations, or `ColorScaleKind::Threshold` with explicit ascending `ColorScaleOptions::thresholds`. Both share the chart and legend paths, preserve repeated cuts, and put equality in the next bucket. `bin_extent` retains unbounded threshold tails; the optional finite `domain` controls the legend window. An empty quantile source maps to the unknown color. Source values and breaks must be finite; no implicit coercion is performed.

`ColorScaleKind::SequentialQuantile` maps full sample ranks through a continuous color ramp while preserving duplicates. Its rank statistics, R7 quantiles and ordinal `rank_colors` have distinct roles; exact legends preserve data-axis jumps under the existing paint budgets. [Sequential quantile guide](docs/sequential-quantile.md)

```rust
# use kartis::*;
let scale = ColorScaleOptions {
    kind: ColorScaleKind::SequentialQuantile,
    ..Default::default()
}.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None])?;
assert_eq!(scale.rank(0.), Some(0.25));
assert_eq!(scale.quantiles(4)?, Some(vec![0., 0., 5., 10., 10.]));
# Ok::<(), ChartError>(())
```

Resolve `OrdinalColorOptions` once and pass the palette to the ordinal color entry points for Bar, Line, and Pie. An explicit key domain preserves color assignments when data is reordered or hidden. `PieColorStyles` can inherit and modify slice paint or select a label color based on its contrast against the rendered background.

```rust
use kartis::*;
let mut scene = ChartConfig::default().scene()?;
let mut mark = Mark::new("volume", Geometry::Rect {
    x: 60., y: 40., width: 200., height: 180.,
}, Color(30, 120, 180, 255));
mark.fill_paint = Some(FillPaint::Linear {
    start: [60., 40.], end: [60., 220.],
    stops: vec![GradientStop::new(0., Color(30, 170, 170, 255)),
                GradientStop::new(1., Color(45, 75, 170, 255))],
});
scene.marks.push(mark);
let svg = render::to_svg(&scene)?;
# Ok::<(), ChartError>(())
```

Use `ValueFormat` and `FormatLocale` for numeric labels and tooltips.

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let locale = FormatLocale::default();
let thousands = format_prefix(".1", 1_000., &locale)?;
assert_eq!(thousands.format(2_500.)?, "2.5k");
assert_eq!(format_text(">8c", "Rust", &locale)?, "    Rust");
# Ok(())
# }
```

UTC coordinates use integral epoch milliseconds. Date and time formatting supports explicit locales and offsets. The `timezone` feature enables IANA zones, civil calendar intervals, and calendar aggregation.

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let format = ValueFormat::date_time(
    "%Y年%-m月%-d日 %H:%M %Z", &TimeLocale::zh_cn(), TimeZone::fixed_offset(480)?,
)?;
assert_eq!(format.format(0.)?, "1970年1月1日 08:00 +0800");
# Ok(())
# }
```

### Tree, parallel coordinates, and networks

Tree layouts offer configurable link curves, node geometry, and labels. Hierarchy layouts can focus on a subtree. Explicit graph relations support highlighting in Tree, Network, Chord, and Sankey.

`HierarchyFocusMotion` animates subtree changes in treemap, circle packing, and icicle layouts. It exposes the accepted target layout separately from the sampled geometry. `frame_interaction` pairs the current frame with typed data and anchors for picking.

`HierarchyLayout::label_candidates` exposes stable labels for measurement. Pass their actual rotated ink bounds to `place_labels` to apply priorities, alternative offsets, obstacles, and controlled hiding. Labels stay within their original node or plot clips. Unplaceable labels can be restored on a later placement. With `raster`, `FontRegistry::measure_label` supplies bounds from the registered fonts.

```rust
use kartis::*;
let data = [HierarchyDatum::new("root", None, 0.),
            HierarchyDatum::new("leaf", Some("root".into()), 1.)];
let mut options = TreeOptions::default();
options.style.links.curve = TreeLinkCurve::Step;
options.style.leaves = Some(TreeNodeStyle {
    shape: TreeNodeShape::Square, radius: Some(6.), ..Default::default()
});
let scene = tree_with_options(&ChartConfig::default(), &data, &options)?;
assert!(matches!(&scene.marks.iter().find(|m| m.id == "node:leaf").unwrap().geometry,
                 Geometry::Rect { .. }));
# Ok::<(), ChartError>(())
```

Parallel coordinates give each dimension its own scale and retain gaps for missing values.

`parallel_layout_with_scale_options` accepts independent bounds and continuous scale options for each dimension. It returns the resolved scales, original row values, projected vertices, and an interaction binding. Hiding a group preserves the inferred domains.

```rust
use kartis::*;
let mut cost = ParallelDimension::new("Cost");
cost.domain = Some([0., 100.]);
let mut rate = ParallelDimension::new("Rate");
rate.scale = ScaleKind::Log;
rate.domain = Some([1., 1000.]);
let rows = [ParallelDatum::new("first", vec![Some(25.), Some(8.)])];
let scene = parallel_coordinates_with_options(
    &ChartConfig::default(), &[cost, rate], &rows, &Default::default(),
)?;
assert!(scene.marks.iter().any(|m| m.series == "first"));
# Ok::<(), ChartError>(())
```

`ForceSimulation` and `NetworkDragController` support node dragging with explicit world-to-client mapping.

```rust
use kartis::*;
let config = ChartConfig::default();
let mut simulation = ForceSimulation::new(&[NetworkNode::new("A")], &[], Default::default())?;
simulation.set_position("A", [0., 0.])?;
let mapping = NetworkDragMapping::from_config(&config,
    ClipRect { x: 0., y: 0., width: config.width, height: config.height },
    ZoomTransform::default())?;
let mut drag = NetworkDragController::new(simulation, Default::default())?;
drag.begin(1, "A", mapping.world_to_client([0., 0.])?, &mapping)?;
drag.end(1, mapping.world_to_client([40., -20.])?, &mapping)?;
assert!(drag.active().is_none());
assert!((drag.simulation().node("A").unwrap().position[0] - 40.).abs() < 1e-9);
# Ok::<(), ChartError>(())
```

Use `ForceSimulation::scene_with_glyphs` to style nodes by ID and `NetworkDragController::begin_with_glyphs` to start a drag on the painted contour. Glyph size is independent of the simulation radius. With the `web` feature, `BrowserNetwork::mount_with_glyphs` connects the same geometry to live simulation and pointer input.

## Rendering and interaction

| Feature | Enables |
| --- | --- |
| Default | Rust layouts, standalone SVG, Canvas script export, and semantic Glory charts |
| `canvas` | Rust/Wasm drawing into a browser Canvas |
| `web` | `BrowserChart` with retained SVG or Canvas, input handling, and animation |
| `raster` | CPU RGBA and PNG, registered fonts, glyph outlines, and raster widgets |
| `raster-bidi` | Opt-in single-line bidirectional shaping for CPU raster and glyph outlines |
| `geojson` | GeoJSON input |
| `timezone` | IANA time zone data and zoned calendar operations |

Use `render::to_svg` for a standalone document or `render::to_svg_with_namespace` for multiple inline charts. `render::to_canvas_script` exports drawing commands for a supplied canvas. `render::draw_canvas` draws a scene into a browser canvas. Implement `render::Canvas` to supply another drawing backend.

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let mut options = PieOptions { inner_radius: 0.6, ..Default::default() };
options.style.corner_radius = 8.;
options.style.arc_labels.legacy_fit = false;
options.style.arc_labels.text = PieLabelText::Percent(ValueFormat::Percent(0));
options.style.link_labels.enabled = true;
let scene = pie(&ChartConfig::default(), &[RadialDatum::new("A", 40.), RadialDatum::new("B", 60.)],
                &options)?;
let svg = render::to_svg(&scene)?;
assert!(svg.contains("<svg"));
# Ok(())
# }
```

Glory’s semantic chart provides a legend, data controls, keyboard navigation, and tooltips. Use `LegendOptions` and `TooltipOptions` to customize presentation. Applications can provide legend and tooltip widgets. Legend filtering respects series-owned labels, and an application can request layout again when visibility changes.

`Chart::data_control_labels` overrides button text by stable mark ID. `BrowserChart` uses these labels to display typed missing values in its data controls.

Use `Chart::plot_widgets` to attach a Glory widget to a data mark in a measured plot rectangle. Slots preserve widget identity across data updates and reordering. A partially clipped slot stays visible within its clip and becomes inert.

Raster charts support the same live widget slots over their PNG image. Slots follow `RasterOptions::view` independently of pixel ratio. Scene, slot layout, image options, and image publication share admission checks. Rectangular masks and fully contained or excluded path slots are supported; partially masked arbitrary paths require the browser mask host. Live widgets remain separate from exported Scene pixels.

Use `BrowserChart::mount_with_plot_widgets` for the same kind of widget over retained SVG or Canvas. Slots follow camera transforms and responsive relayout and honor the datum's rectangular or path clip. `set_layout_with_plot_widgets` submits geometry, typed interaction data, and slots together.

`BrowserChart` supports pointer and keyboard input, hover and selection, nearest and slice picking, zoom, pan, and brush. Domain viewport callbacks rebuild chart geometry while keeping axes in place. Submit a `SceneInteraction` to update the scene, plot, and typed interaction data together. Motion can animate chart properties and geometry with explicit contour options.

`EditableBrush` adds persistent X, Y, or XY selections, movement, edge and corner resizing, centered adjustment, direction locking, and temporary movement with Space. `SurfaceModel` keeps its selection in logical viewport pixels and resolves current typed data anchors once when reporting selected IDs. Configure an explicit extent, then use the same checked controller in SVG, Canvas, and raster hosts. Programmatic updates retire older gesture sessions; failed updates retain the accepted frame. [Editable brush](docs/editable-brush.md)

```rust
use kartis::{surface::{SurfaceModel, SurfaceOptions}, *};
let scene = ChartConfig::default().scene()?;
let extent = Brush::from_points((0., 0.), (scene.width, scene.height))?;
let mut surface = SurfaceModel::new(scene, SurfaceOptions::default())?;
surface.configure_editable_brush(
    Some(EditableBrushOptions { extent, ..Default::default() }),
    BrushReconfigure::Preserve,
)?;
surface.set_brush_selection(Some(Brush::from_points((40., 30.), (160., 100.))?))?;
surface.apply_editable_brush(EditableBrushInput::Nudge {
    target: BrushTarget::Selection, delta: (10., 0.),
})?;
assert_eq!(surface.editable_brush().unwrap().selection().unwrap().x0, 50.);
# Ok::<(), ChartError>(())
```

`Theme::font` sets the chart's family list and weight. `Scene::typography` carries inherited defaults, per-label overrides, and legend and tooltip roles through drawing, filtering, and animation. `PresentationText::font` supplies local presentation overrides. SVG, Canvas, and semantic widgets use the same resolved font; `Chart::font_text_measurer` supplies that font to application measurement. Measured Pie, Hierarchy, Sunburst, and AreaBump candidates include their font and reject measurements from a different font. Their `with_label_font` method changes managed text before measuring.

For precise ordinary Text placement, use `Chart::semantic_text_measurer(profile, measure)`. The host returns a bound `SemanticTextMeasurement` containing its real advance, line-box height, alphabetic baseline and ink bounds. Placement and collision share those metrics, including bearings and descenders; the original Text node remains live. `SemanticTextProfile::line_height` requests a font-size ratio. Increment its `revision` after font resources or measurement context change; stale or failed measurements reject the frame together with scene and layout updates. Without this adapter, ordinary Text retains its compatibility placement.

With `web`, `BrowserSemanticTextMeasurer::new_in` measures an actual HTML line in the chart's layout context. Move it into the chart's measurement callback and wait for fonts before mounting. Its probe is removed on Drop. Custom hosts can provide their own measured baseline; the native example demonstrates actual Apace line metrics. Rotated text continues to use the explicit outline or painted path APIs.

The `raster` feature supports `render_raster`, `RasterCanvas`, and `chart(scene).raster(fonts, options)`. Register font bytes with `FontRegistry` for text rendering. For explicit family selection, use `bind_face(FontId, FaceDescriptor)` to bind each supplied static face's actual weight to a named or generic family. Family fallback and CSS weight matching choose from those bindings without synthetic bolding or system font discovery. Measurement, CPU drawing, and outlined or painted paths share the selected bytes. Unbound family entries are skipped; a request with no bound family rejects the frame. Without font declarations, registration-order fallback remains available. Raster widgets publish PNG images through the host image source. [CPU export example](examples/raster_export.rs) · [Raster widget example](examples/browser-raster/README.md)

`RasterSurface::mount_layout` and `RasterSurfaceHandle::set_layout` retain a layout's typed interaction binding alongside its image. Controls distinguish missing values from zero, tooltips use source rows, and explicit anchors support custom shapes. The image, controls, plot, and binding update together after validation and successful image publication. Applications supply host pointer bounds to `dispatch_host`.

With `raster-bidi`, use `FontRegistry::with_bidi` to select automatic, left-to-right, or right-to-left paragraph direction for single-line labels. Register a font that covers each contextual script span, such as joined Arabic text. Raster output and outlined labels share the resulting glyph positions.

Use `FontRegistry::with_writing_mode(RasterWritingMode::Vertical)` for one upright, top-to-bottom line. Measurement, raster output, and glyph outlines use the same vertical glyph positions. Set `OutlineTextOptions::rotated_only` to `false` when outlining labels with zero rotation.

`FontRegistry::with_vertical_layout` supports explicit columns progressing left or right and Unicode mixed glyph orientation. CJK glyphs remain upright; sideways runs use horizontal shaping and rotation, with vertical alternates where available. `measure_layout` returns column advances and two-dimensional bounds. Automatic wrapping and complete CSS text layout remain outside this profile.

Use `FontRegistry::with_color_glyphs(ColorGlyphMode::ColrV0 { palette })` for COLR v0/CPAL solid layers. CPU text and `Chart::painted_labels` use the same colored paths, including palette alpha, holes, vertical shaping, and rotation. Transparent foreground color affects its own layers while other palette layers remain visible. The default font profile rejects unsupported color glyphs; COLR v1 paint graphs and bitmap glyphs remain separate work.

`scene_with_painted_labels` makes a text-to-path copy for standalone SVG, Canvas, or CPU export. It preserves business marks, paint order, clipping, and series ownership, and validates generated IDs and resource budgets. Keep the original Scene when original text or editing is needed. This conversion exports font paths rather than live widget content.

The desktop example uses Glory’s vector graphics in Apace and a CPU raster widget in Wry. `Chart::basic_data_controls` keeps focus and activation available on hosts that support standard buttons. [Desktop example](examples/native-raster/README.md#desktop-window)

## Scope

The public API covers selected D3 and Nivo layout and interaction profiles. Supported options vary by chart family. The semantic Graphic, SVG, Canvas, and CPU backends have distinct capabilities. GPU and worker rendering are deferred. [Feature gaps](docs/feature-gaps.md) · [Verification](docs/verification.md) · [Cross-browser verification](docs/cross-browser-verification.md)

Licensed under the Apache License, Version 2.0. [License](./LICENSE)

Spherical geometry provides checked distance, streamed length, oriented area, sampled circles, centroid and containment over borrowed typed inputs. [Spherical guide](docs/geo-spherical.md)

Standalone CanvasScript exports also offer explicit main-thread Offscreen2D, cumulative buffer admission and an availability policy. [CanvasScript guide](docs/canvas-script.md)

Bar category bands offer explicit shared rounding, alignment, padding and direction. [Category scale guide](docs/bar-category-scale.md)

Typed factories customize each Bar or HeatMap mark's geometry, gradient, pattern, stroke and opacity while retaining original values and interaction sites. Controlled hover validates the accepted source and painted frame. [Custom Bar guide](docs/bar-custom-geometry.md) · [Custom HeatMap guide](docs/heatmap-custom.md)

Bar and HeatMap value labels can opt into shared inherited colors, ordered modifiers and contrast selection. Label colors resolve from the layout palette before custom paints and retain controlled-hover alpha. [Inherited label colors](docs/inherited-label-colors.md)

Bar and HeatMap ordinary borders can opt into the same inherited-color roles while preserving configured stroke width and cap. Profiles resolve from the numeric palette before custom factories; setting a profile alone does not enable a border. Normal Canvas/CPU rendering applies mark opacity once to the combined fill and stroke. Glory Chart supports rectangular label clips and clip-only updates; Circle and Path label masks remain unsupported. [Border color roles](docs/border-color-roles.md)

Typed Bar and HeatMap label factories customize text from the original source without changing raw values or tooltips. They compose with geometry factories and preserve owned labels through controlled hover. Bar also offers explicit zero-value-axis-extent labels while keeping its default skip policy. [Typed label guide](docs/typed-labels.md)

RadialBar labels can inherit category colors; Sunburst accepts a typed label-text factory before measurement; BoxPlot exposes explicit D3 group-band rounding, alignment and padding. [Label and band configuration](docs/chart-labels-and-bands.md)

Bump endpoint labels accept independent style overrides, Radar point paints accept inherited color roles, and BoxPlot value bounds can be automatic or fixed independently. [Endpoint, point and bound configuration](docs/endpoint-point-bounds.md)

BoxPlot can infer automatic bounds from summary quantiles; Bar supports source category legends and explicit series order, with typed legend roles and opt-in category actions. [Summary domains and legend actions](docs/summary-index-legend.md)

RadialBar segment borders and BoxPlot box, median and whisker strokes can opt into inherited-color profiles while retaining fixed defaults and stroke geometry. Role alpha stays separate from segment or fill opacity. [Stroke color guide](docs/radial-box-strokes.md)

BoxPlot supports Group or SubGroup palette identities and controlled highlighting of every box sharing the active group or subgroup. Whole-box opacity composites once across SVG, Canvas, CPU raster and semantic Glory charts. [BoxPlot color and hover guide](docs/boxplot-color-and-hover.md)

BoxPlot legend captions can be overridden by stable raw group or subgroup keys while retaining identity, role, palette and controlled association. Empty and repeated captions keep distinct entries. [Legend caption guide](docs/boxplot-legend-labels.md)

Opt into whole-part painted picking with `BoxPlotInteraction::whole_parts_scene_interaction()`: borders, medians, whiskers, caps and outliers share one logical site per box while retaining physical mark IDs and accepted outlier source snapshots. `chart_layout` and the Browser/Raster layout APIs accept the Scene and binding together; the original body-site entries remain available. [Whole-part pointer guide](docs/whole-part-pointer.md)

BoxPlot offers an opt-in per-summary String legend factory with `ByLabel` deduplication, stable full-domain ordering and typed row-to-series or category targets. Default `Keys` preserves key-based captions. Controlled hover reuses cached rows; explicit factory source replacement rebuilds them. [Per-summary legend factory guide](docs/boxplot-legend-factory.md)

BoxPlot accepts borrowed per-summary RGBA and ByLabel caption factories through `BoxPlotFactories`. Body, default outliers, inherited stroke roles and first-representative legends share the resolved source color; controlled hover reuses the cache, and source replacement explicitly supplies the required factories. [Source color factory guide](docs/boxplot-color-factory.md)

Calendar FullYears layouts support explicit month spacing in both orientations, shared by day cells, month outlines and labels. Zero preserves the existing layout. [Calendar month spacing](docs/calendar-month-spacing.md)

Radar point labels accept a borrowed text factory with axis and series identities, raw values and formatted values. Label and geometry factories can be combined with shared value scales. [Radar point labels](docs/radar-point-labels.md)
