# Albers USA: fitted maps and region overlays

[简体中文](geo-albers-usa.zh-CN.md)

**Status: Unreleased.** Dedicated fitted maps, component-preserving lines and
graticules, point/inverse primitives, planar rectangle clipping, selected-region
symbol/glyph and location-label masks, and host inverse adapters are available.
The input and numerical limits below apply; these APIs do not promise arbitrary
spherical topology or bit-identical D3 subdivision.
Published Kartis 0.1.0 does not contain these APIs.

```rust
use kartis::*;

let projection = GeoAlbersUsaProjection::default();
let projected = projection.project([-149.9, 61.2])?.expect("visible Anchorage");
assert_eq!(projected.region, GeoUsaRegion::Alaska);
let geographic = projection
    .invert_in_region(projected.region, projected.position)?
    .expect("same-region inverse");
# Ok::<(), ChartError>(())
```

`new(scale, translate)` accepts pixels per composite raw unit and a screen
origin; Default uses D3's scale 1070 and origin `[480, 250]`. Fixed children use
the original equal-area parallels, intrinsic longitude frames and raw projected
origins. Alaska uses 0.35 of the base scale; Hawaii uses the base scale. No camera,
standard-parallel or arbitrary clip-angle controls are introduced.

`project` selects the first visible child in Lower48/Alaska/Hawaii order and
returns `GeoUsaProjectedPoint { position, region }`. `project_in_region` projects
through one specified component. Region clipping is inclusive, with the original
Alaska/Hawaii window inset of **1e-6 final pixels**, independent of scale.
`region_clip` returns that component's rectangle. Invalid input, arithmetic or
collapsed/nonrepresentable windows reject instead of returning an incomplete
three-region frame. These projection windows are not political country masks.

`invert` follows D3's half-open inset selector, then checks the selected region;
it does not search other regions on failure. It returns `GeoUsaInverse` with
longitude/latitude and the selected region. `invert_in_region` retains known
provenance. Composite image overlaps mean a global `invert(project(p))` need not
recover the original geographic point. Points outside the selected visible
footprint return None; this differs from D3's unrestricted algebraic inverse on
arbitrary far-away pixels. Membership uses the requested pixel's exact region
rectangle, then checked inverse and an unclipped same-region residual. Rounding
the resulting longitude/latitude can place its strict forward result one ULP
outside a boundary; that does not move or enlarge the forward clip. The checked
conic domain/residual guard remains alongside native expression ordering.

## Planar rectangle clipping

```rust
use kartis::*;

let rings = vec![vec![[-5., -5.], [15., -5.], [15., 15.], [-5., 15.]]];
let result = clip_geo_planar_polygon(
    &rings,
    ClipRect { x: 0., y: 0., width: 10., height: 10. },
    &GeoRectClipOptions::default(),
)?;
assert!(!result.rings.is_empty());
# Ok::<(), ChartError>(())
```

`clip_geo_planar_polygon` clips a nonzero-winding polygon stream with boundary
rejoining; `clip_geo_planar_lines` clips independent open lines without connecting
their fragments. This is projected planar clipping, not geographic latitude or
antimeridian clipping. Concave splits, boundary-crossing holes and whole-window
coverage require this topology stage. Output contours retain native winding,
closing duplicates and degenerate contacts; their list positions do not identify
exteriors versus holes. Polygon inputs need at least three vertices per ring;
an explicit repeated closing point is accepted.

`GeoRectClipOptions` bounds final points and work, including empty input headers,
scanning, sorting and graph traversal. Intermediate buffered points are bounded
by twice `max_points`. Stats expose input/buffer/output point counts and work.
Coordinates must be finite. Crossing segments use the native +/-1e9 clamp;
nonrepresentable winding arithmetic rejects. Errors return no partial result.
Consumers must share budgets across the three components and fitting passes.

## Fitted maps

```rust
use kartis::*;

let mut layout = geographic_usa_map_layout(
    &ChartConfig::default(),
    &[],
    &GeoUsaMapOptions::default(),
)?;
layout.add_symbols(
    &[GeoSymbol::new("Anchorage", [-149.9, 61.2], Some(7.))],
    &GeoSymbolOptions { clip_to_projection: true, ..Default::default() },
)?;
let projected = layout.projection.project_usa([-149.9, 61.2])?.unwrap();
assert_eq!(projected.region, GeoUsaRegion::Alaska);
# Ok::<(), ChartError>(())
```

`GeoUsaMapOptions` provides `fit`, `padding`, `scale`, `translate`, `fill`,
`border`, `border_width` and adaptive `path` options. It leaves the existing
`GeoProjection` enum and `GeoMapOptions` fields intact. `World` fits the fixed
normalized composite rectangle. `Data` measures all three actually clipped
polygon streams at temporary scale 150 and origin zero, then rebuilds the final
pixel clips. `translate` places the fitted bounds' center; for Data fit the native
D3 origin also accounts for `fitted_raw_center()`. Pass the retained context to
overlays instead of fitting them separately.

Each polygon streams through three continuous conic children before rectangle
postclipping. One source feature retains one compound `map:{id}` mark, its
series, label and value, including disconnected pieces and holes. Native contour
winding is retained. `choropleth_usa_layout` supplies the default value ramp;
`choropleth_usa_with_color_scale` and `_with_color_profile` resolve a shared color
domain from all source values, including clipped features and missing values.

Sampling emissions, postclip output and work share the caller's `path` budget
across components and both Data fitting/final passes. `usa_map_stats()` reports
aggregate points, work, depth and passes. Preparation/topology work is admitted
conservatively before allocation. Final Scene command limits also apply. This
budget is stricter than simply granting each component a separate limit. The
adaptive sampler differs from D3's subdivision algorithm; it does not promise
identical path point counts or exact floating-point fit bounds.

## Selected-region overlays and picking

The retained context's existing `add_symbols` and `add_glyphs` use the composite's
forward priority. `clip_to_projection` masks the complete original fill/stroke
to that selected region's rectangle. `clip_labels_to_projection` independently
masks newly added location labels; legends remain complete. Plot clipping is
separate. Glyph geometry, source indexes, nullable values and typed anchors are
preserved. No global union mask substitutes for a selected component. The mark
mask uses native `Mark.clip`, intersected with the plot rectangle when requested,
so the default profile also mounts as a Glory Graphic. Label rectangles retain
the shared editable-label clipping pipeline.

`project_usa`, `invert_usa` and `invert_usa_in_region` expose region-aware
coordinates on the fitted context; they reject ordinary contexts. Browser hosts
provide `invert_usa_at_client` / `invert_usa_in_region_at_client`; native raster
hosts provide the corresponding `_at_host` methods. These reuse the accepted
coordinate conversion and remove Scene pan/zoom once. Retain the context with
its accepted layout: matching dimensions cannot detect a stale same-sized map.

The general `project`, `invert` and `invert_in_plot` also dispatch to the
composite. `projection_disk` returns None. A single `projection_clip_path` rejects
because it cannot provide inset isolation. The older `add_lines` and
`add_graticule` signatures explicitly reject USA contexts; use the dedicated
methods below to retain component geometry and interaction ownership.

## Lines, graticules and canonical interaction

```rust
use kartis::*;

let mut layout = geographic_usa_map_layout(
    &ChartConfig::default(), &[], &GeoUsaMapOptions::default(),
)?;
layout.add_usa_graticule(&GeoGraticuleOptions::default(), &GeoStreamOptions::default())?;
let mut route = GeoLineFeature::new("route", vec![]);
route.lines = vec![
    vec![[-124., 38.], [-119., 38.]],
    vec![[-152., 61.], [-146., 61.]],
    vec![[-159., 21.], [-156., 21.]],
];
let metadata = layout.add_usa_lines(&[route], &GeoStreamOptions::default())?;
let frame = metadata.into_scene_interaction(layout.scene)?;
let widget = chart_layout(frame);
# Ok::<(), ChartError>(())
```

`add_usa_lines` reuses the fitted map context. Each source's open components
stream independently through three continuous children before pixel rectangle
postclipping. No composite point samples are connected across insets. Each
visible region has a real path and its own full-ink rectangle. `clip_projection`
controls stroke/cap masking; centerline postclipping always applies.
`clip_plot` is independent, and requesting both uses their exact rectangle
intersection, including an empty intersection. Dash, opacity and per-source
style remain actual paint. `behind` and existing explicit paint order are
preserved transactionally. `add_usa_graticule` uses the existing major/minor
source generator, always noninteractive.

`GeoUsaStreamMetadata.lines` retains source indexes, IDs, raw nullable values and
per-region `parts`. `source_series` preserves the input grouping string;
effective `series` falls back to the source ID when it is empty. An empty label
also falls back to ID for the real mark/control, while metadata `label` remains
the original string. Numeric formatting is stored separately and applied once.
An invisible source has no fabricated mark or control. The first visible real
part uses `geo-line:{id}`; additional regions alias that canonical site. Its
typed anchor is the first visible Move endpoint, not a geographic centroid.

`interaction_binding(scene)` checks the authored layer's geometry, style,
clipping, metadata and paint order. `into_scene_interaction(scene)` binds this
layer for Glory/browser/raster hosts using explicit `PaintedGeometry` and
Geometry picking. Each source has one keyboard control; all visible regional
parts resolve to it. Unrelated map/glyph marks stay painted but are not silently
added to this binding. Nearest/slice modes are not this profile. Recompute the
layer metadata when replacing its authored geometry or style; host-managed
animation retains its own accepted-frame rules.

Source, coarse, adaptive and postclip points share one `path.max_points` budget:
`source_points + coarse_points + sampling.points + clip_stats.output_points`.
Total work is `sampling.work + clip_stats.work`; input and empty-row headers are
charged too. Scene-wide command/mark limits still apply. Any failure leaves the
caller Scene unchanged. Bound Glory default tooltips retain nullable/formatted
source rows even when a live mark scalar changes, and stationary-pointer
metadata-only updates invalidate the readout. The existing custom formatter
that accepts `Mark` values keeps its finite-scalar compatibility contract.

## Verification limits

Original child streams and composite event records are both retained. Six
cross-inset reference cases make the original D3 composite interleave child
`lineStart` events. Kartis keeps independent regional paths; it does not claim
that flattening those interleaved events is an equivalent no-bridge geometry.
Material contours and bidirectional segment distances are checked separately.
Three roughly `1e-13`-pixel corner contacts differ between the samplers and are
preserved as diagnostics, not relabeled exact matches. Ambiguous antipodal arcs,
nonrepresentable coordinates and exhausted budgets reject explicitly.

Native/browser host input, region provenance, actual SVG/Canvas/CPU ink,
editable browser labels and canonical control transactions have dedicated
acceptance. Pixel samples avoid antialiasing edges; they do not establish
whole-image equality or physical-device behavior. See
[verification](verification.md#development-albers-usa-streams-and-host-integration)
for the measured scope and retained failures. Broader geographic topology,
other feature gaps and GPU/worker rendering remain separate work.

The reference is d3-geo 3.1.1 `projection/albersUsa.js`, its conic children and
`clip/{rectangle,line,buffer,rejoin}.js` (ISC). Frozen reference inventories and
generators are in `tests/fixtures/geo-usa-kernel`, `tests/fixtures/geo-rect-clip`,
`scripts/geo-usa-reference.mjs` and `scripts/geo-rect-clip-reference.mjs`.
The separate map stream/fit oracle is generated by
`scripts/geo-usa-map-reference.mjs` in `tests/fixtures/geo-usa-map`.
The line/graticule oracle is `scripts/geo-usa-stream-reference.mjs` and
`tests/fixtures/geo-usa-stream`.
